//! The Unix side of `sso_cli` (261011-sso-login): the AWS CLI is started
//! in a new process group of its own, and the whole group is stopped with
//! SIGTERM, then SIGKILL after the grace time (BR3.3, NFR4). macOS is the
//! distributed platform; CI also builds and tests on Linux.

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// A regular file with an execute bit (BR2.1).
pub(super) fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// Starts the CLI: no shell, the arguments as given, standard input closed,
/// both outputs piped, the environment inherited, a new process group
/// (BR2.2, BR2.4). A CLI whose pid cannot be a group of its own is stopped
/// again and reported as an error, since it could never be signalled.
pub(super) fn spawn_cli(cli: &Path, args: &[String]) -> io::Result<(Child, ProcessGroup)> {
    let mut child = Command::new(cli)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let Some(group) = ProcessGroup::of(&child) else {
        if let Err(error) = child.kill() {
            eprintln!(
                "local-sights: sso login: the AWS CLI could not be stopped ({:?})",
                error.kind()
            );
        }
        if let Err(error) = child.wait() {
            eprintln!(
                "local-sights: sso login: the AWS CLI could not be reaped ({:?})",
                error.kind()
            );
        }
        return Err(io::Error::other("the AWS CLI has no usable process id"));
    };
    Ok((child, group))
}

/// The CLI's process group: its own pid (`process_group(0)`), always > 1,
/// so a signal to it can never reach this app's own group (0) or every
/// process (1, -1).
#[derive(Debug)]
pub(super) struct ProcessGroup(libc::pid_t);

impl ProcessGroup {
    /// `None` for a pid that cannot be a group of its own.
    fn of(child: &Child) -> Option<Self> {
        libc::pid_t::try_from(child.id())
            .ok()
            .filter(|pid| *pid > 1)
            .map(Self)
    }

    /// SIGTERM to the group, then up to `grace` for it to end (`reap` lets
    /// the caller collect the leader meanwhile), then SIGKILL to what is
    /// left (BR3.3).
    pub(super) fn terminate(&self, grace: Duration, mut reap: impl FnMut() -> bool) {
        self.signal(libc::SIGTERM);
        let deadline = Instant::now() + grace;
        loop {
            reap();
            if !self.exists() {
                return;
            }
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        self.signal(libc::SIGKILL);
        reap();
    }

    /// Whether any process of the group still exists.
    pub(super) fn exists(&self) -> bool {
        // SAFETY: signal 0 only checks that the group exists; nothing is
        // sent. The group is > 1 by construction.
        unsafe { libc::killpg(self.0, 0) == 0 }
    }

    /// The CLI ended by itself: nothing to do on Unix, what it started
    /// (a browser) was never part of the stop.
    pub(super) fn release(&self) {}

    /// Sends `signal` to every process of the group.
    fn signal(&self, signal: libc::c_int) {
        // SAFETY: killpg only sends a signal; the group is the one the CLI
        // was started in, > 1 by construction.
        let result = unsafe { libc::killpg(self.0, signal) };
        if result != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                eprintln!("local-sights: sso login: the AWS CLI could not be signalled ({error})");
            }
        }
    }
}
