//! The Windows side of `sso_cli` (261015-windows-support FR4.2-FR4.4):
//! the AWS CLI is started without a console window and put into a job
//! object of its own, so that it and everything it starts are stopped
//! together.
//!
//! - The job is made with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` before the
//!   CLI is assigned to it: when the last handle to the job closes, which
//!   also happens when this app ends abnormally, the OS ends every process
//!   still in it (FR4.4; nothing of the CLI survives a crash of the app).
//! - Stopping is `TerminateJobObject`: there is no gentle stop like
//!   SIGTERM on Windows, so the grace time only bounds the wait for the job
//!   to report no active process.
//! - A CLI that ends by itself has the kill-on-close limit removed before
//!   the handle is dropped (`release`), so a browser it started and that is
//!   still in the job is left alone, as on Unix.
//!
//! The functions come from the `windows-sys` crate (the version Tauri
//! already builds). Every call is a thin wrapper around one documented
//! kernel32 function; the handle is closed in `Drop`.

use std::ffi::c_void;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

/// The exit code given to the processes of a terminated job.
const TERMINATED_EXIT_CODE: u32 = 1;

/// A regular file whose extension is `exe` (in any case); Windows has no
/// execute bits (FR4.1).
pub(super) fn is_executable_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        && std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// Starts the CLI: no shell, the arguments as given, standard input closed,
/// both outputs piped, the environment inherited, no console window
/// (`CREATE_NO_WINDOW`, FR4.2), inside a new job object (FR4.3). The job is
/// made first, so a CLI that cannot be put into it is stopped at once and
/// reported as an error rather than left running on its own.
pub(super) fn spawn_cli(cli: &Path, args: &[String]) -> io::Result<(Child, ProcessGroup)> {
    let group = ProcessGroup::new()?;
    let mut child = Command::new(cli)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    if let Err(error) = group.assign(&child) {
        if let Err(kill_error) = child.kill() {
            eprintln!(
                "local-sights: sso login: the AWS CLI could not be stopped ({:?})",
                kill_error.kind()
            );
        }
        if let Err(wait_error) = child.wait() {
            eprintln!(
                "local-sights: sso login: the AWS CLI could not be reaped ({:?})",
                wait_error.kind()
            );
        }
        return Err(error);
    }
    Ok((child, group))
}

/// The CLI's job object: the CLI and every process it starts.
#[derive(Debug)]
pub(super) struct ProcessGroup {
    job: HANDLE,
}

// SAFETY: a job object handle is a process-wide kernel object; it may be
// used and closed from any thread, and the OS serialises the calls.
unsafe impl Send for ProcessGroup {}
unsafe impl Sync for ProcessGroup {}

impl ProcessGroup {
    /// An unnamed job with the kill-on-close limit set.
    fn new() -> io::Result<Self> {
        // SAFETY: both arguments may be null (default security, no name);
        // the returned handle is owned by this value and closed in Drop.
        let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let group = Self { job };
        group.set_limit_flags(JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE)?;
        Ok(group)
    }

    /// Puts the CLI into the job; its children join automatically.
    fn assign(&self, child: &Child) -> io::Result<()> {
        // SAFETY: both handles are valid for the duration of the call: the
        // job is owned by self, the process handle by `child`.
        let assigned = unsafe { AssignProcessToJobObject(self.job, child.as_raw_handle()) };
        if assigned == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Sets the basic limit flags of the job (the only limit used).
    fn set_limit_flags(&self, flags: u32) -> io::Result<()> {
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = flags;
        // SAFETY: the buffer is a fully initialised extended limit record
        // and its size is passed with it; the class names that record.
        let set = unsafe {
            SetInformationJobObject(
                self.job,
                JobObjectExtendedLimitInformation,
                ptr::from_ref(&limits).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if set == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Ends every process of the job at once, then waits at most `grace`
    /// for the job to report none active (`reap` lets the caller collect
    /// the CLI meanwhile). There is no gentle stop to try first on Windows.
    pub(super) fn terminate(&self, grace: Duration, mut reap: impl FnMut() -> bool) {
        // SAFETY: TerminateJobObject only ends the processes of this job.
        let terminated = unsafe { TerminateJobObject(self.job, TERMINATED_EXIT_CODE) };
        if terminated == 0 {
            let error = io::Error::last_os_error();
            eprintln!("local-sights: sso login: the AWS CLI could not be stopped ({error})");
        }
        let deadline = Instant::now() + grace;
        loop {
            reap();
            if !self.exists() || Instant::now() >= deadline {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Whether any process of the job is still active. A job that cannot
    /// be queried counts as gone, so a stop never waits on it for nothing.
    pub(super) fn exists(&self) -> bool {
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: the buffer is a basic accounting record of the size
        // passed with it, which the class names; the length output is not
        // needed and may be null.
        let queried = unsafe {
            QueryInformationJobObject(
                self.job,
                JobObjectBasicAccountingInformation,
                ptr::from_mut(&mut accounting).cast::<c_void>(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                ptr::null_mut(),
            )
        };
        queried != 0 && accounting.ActiveProcesses > 0
    }

    /// The CLI ended by itself: the kill-on-close limit is removed, so
    /// closing the handle leaves what the CLI started (a browser) alone.
    pub(super) fn release(&self) {
        if let Err(error) = self.set_limit_flags(0) {
            // The limit stays: a browser the CLI started may be closed
            // with the handle. Nothing secret is in the error.
            eprintln!("local-sights: sso login: the job limit could not be removed ({error})");
        }
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // SAFETY: the handle was returned by CreateJobObjectW and is closed
        // exactly once, here.
        let closed = unsafe { CloseHandle(self.job) };
        if closed == 0 {
            let error = io::Error::last_os_error();
            eprintln!("local-sights: sso login: the job handle could not be closed ({error})");
        }
    }
}
