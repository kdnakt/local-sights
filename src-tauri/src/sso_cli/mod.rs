//! Running `aws sso login` of the AWS CLI (261011-sso-login; Windows since
//! 261015-windows-support).
//!
//! The decisions (where to sign in, the arguments, what of the output may
//! be shown) live in `local_sights_core::sso_login`; this module only finds
//! the CLI and runs it:
//!
//! - [`cli_candidates`] and [`locate_cli`]: the first candidate that is an
//!   executable file. On Unix the candidates are `/opt/homebrew/bin/aws`
//!   and `/usr/local/bin/aws`; `PATH` is not used, since an app started
//!   from Finder or the Dock does not get the terminal's `PATH` (BR2.1).
//!   On Windows they are the official installer under `Program Files`,
//!   then `aws.exe` in each absolute `PATH` folder (FR4.1). The path found
//!   is the one started (review R-05).
//! - [`run_login`]: starts the CLI without a shell, with the given
//!   arguments only, standard input closed and the app's environment as is
//!   (BR2.2, BR2.4), inside a process group of its own: a new Unix process
//!   group, or a Windows job object (FR4.2, FR4.3). Standard output and
//!   standard error are read line by line on their own threads; each line
//!   (at most [`MAX_LINE_BYTES`] bytes of it) is handed to
//!   `extract_login_hints` and dropped: only the allow-listed URL and the
//!   code leave this module, nothing is kept or logged (BR3.1, NFR3).
//! - Cancel, the time-out (five minutes, BR3.2) and the app's exit stop the
//!   whole process group (BR3.3, NFR4, FR4.4): on Unix SIGTERM, then
//!   SIGKILL to what is left after the grace time; on Windows the job is
//!   terminated at once, since there is no gentle stop to try first. A CLI
//!   that ends by itself is left alone, so a browser it started is not
//!   closed.
//!
//! Everything that touches the OS (starting, grouping, stopping) is in the
//! `unix` and `windows` modules behind one small interface: `spawn_cli`,
//! `ProcessGroup` and `is_executable_file`. This file holds the decisions
//! and the output handling, which are the same on both.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
use unix as platform;
#[cfg(windows)]
use windows as platform;

use std::ffi::OsStr;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use local_sights_core::sso_login::{
    LoginHints, MAX_LINE_BYTES, SsoLoginOutcome, extract_login_hints,
};

use platform::{ProcessGroup, is_executable_file, spawn_cli};

/// Where the AWS CLI is looked for on Unix, in this order (BR2.1): Homebrew
/// on Apple Silicon, then the official installer.
pub const CLI_CANDIDATES: [&str; 2] = ["/opt/homebrew/bin/aws", "/usr/local/bin/aws"];
/// The official installer's folder under `Program Files` on Windows
/// (261015-windows-support FR4.1).
#[cfg(windows)]
const WINDOWS_CLI_UNDER_PROGRAM_FILES: &str = r"Amazon\AWSCLIV2\aws.exe";
/// `Program Files` when the `ProgramFiles` variable is missing or empty.
#[cfg(windows)]
const DEFAULT_PROGRAM_FILES: &str = r"C:\Program Files";
/// The CLI's file name on Windows, looked for in each `PATH` folder.
#[cfg(windows)]
const WINDOWS_CLI_FILE_NAME: &str = "aws.exe";
/// How long the approval in the browser is waited for (BR3.2).
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// How long a stopped CLI gets to end after SIGTERM before SIGKILL (BR3.3).
/// On Windows it only bounds the wait for the job to be gone.
pub const STOP_GRACE: Duration = Duration::from_secs(2);
/// How often the runner looks at the CLI, the stop request and the clock.
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// Bytes read from the CLI at once.
const READ_CHUNK: usize = 1024;
/// How long the last output lines are waited for after the CLI ended.
const DRAIN_LIMIT: Duration = Duration::from_millis(500);

/// The paths the AWS CLI is looked for at, in order (BR2.1, FR4.1).
///
/// Unix: the two fixed [`CLI_CANDIDATES`]; `program_files` and `path` are
/// ignored, since an app started from Finder does not get the terminal's
/// `PATH`. Windows: the official installer under `program_files` (the
/// `ProgramFiles` variable, `C:\Program Files` when missing), then `aws.exe`
/// in each absolute folder of `path` (the `PATH` variable) in order; empty
/// and relative entries are skipped, so every candidate is an absolute path
/// and the one found is the one started (review R-05).
#[cfg(unix)]
pub fn cli_candidates(_program_files: Option<&OsStr>, _path: Option<&OsStr>) -> Vec<PathBuf> {
    CLI_CANDIDATES.iter().map(PathBuf::from).collect()
}

/// See the Unix version above.
#[cfg(windows)]
pub fn cli_candidates(program_files: Option<&OsStr>, path: Option<&OsStr>) -> Vec<PathBuf> {
    let program_files = program_files
        .filter(|value| !value.is_empty())
        .map_or_else(|| PathBuf::from(DEFAULT_PROGRAM_FILES), PathBuf::from);
    let mut candidates = vec![program_files.join(WINDOWS_CLI_UNDER_PROGRAM_FILES)];
    if let Some(path) = path {
        candidates.extend(
            std::env::split_paths(path)
                .filter(|folder| folder.is_absolute())
                .map(|folder| folder.join(WINDOWS_CLI_FILE_NAME)),
        );
    }
    candidates
}

/// The first candidate that is an executable regular file (symbolic links
/// are followed, as Homebrew's `aws` is one), or `None` (BR2.1). Unix
/// looks at the execute bits, Windows at the `.exe` extension.
pub fn locate_cli<P: AsRef<Path>>(candidates: &[P]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(AsRef::as_ref)
        .find(|path| is_executable_file(path))
        .map(Path::to_path_buf)
}

/// How long [`run_login`] waits; the app uses [`LOGIN_TIMEOUT`] and
/// [`STOP_GRACE`], tests use shorter times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginTimes {
    /// The time-out of the whole login (BR3.2).
    pub timeout: Duration,
    /// The time between SIGTERM and SIGKILL (BR3.3); on Windows the most
    /// the stop waits for the job to be gone.
    pub grace: Duration,
}

impl Default for LoginTimes {
    fn default() -> Self {
        Self {
            timeout: LOGIN_TIMEOUT,
            grace: STOP_GRACE,
        }
    }
}

/// Stops one running login from another thread: [`LoginControl::request_stop`]
/// for [Cancel] (the runner stops the CLI and reports Cancelled), and
/// [`LoginControl::stop_now`] for the app's exit, which also stops the CLI
/// itself before returning, since the runner thread ends with the app.
#[derive(Debug, Default)]
pub struct LoginControl {
    state: Mutex<ControlState>,
}

#[derive(Debug, Default)]
struct ControlState {
    stop_requested: bool,
    /// The CLI's process group, once it runs; shared with the runner.
    process_group: Option<Arc<ProcessGroup>>,
}

impl LoginControl {
    /// A control for one login.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// [Cancel]: asks the runner to stop the CLI.
    pub fn request_stop(&self) {
        self.lock().stop_requested = true;
    }

    /// The app ends: asks the runner to stop and stops the CLI's process
    /// group right here, waiting at most `grace` (BR3.3).
    pub fn stop_now(&self, grace: Duration) {
        let process_group = {
            let mut state = self.lock();
            state.stop_requested = true;
            state.process_group.clone()
        };
        if let Some(group) = process_group {
            group.terminate(grace, || false);
        }
    }

    fn stop_requested(&self) -> bool {
        self.lock().stop_requested
    }

    /// Records the CLI's process group; `false` when a stop was asked
    /// before, so the caller stops it at once. Under one lock with
    /// [`LoginControl::stop_now`], so a stop never misses the CLI.
    fn register(&self, process_group: Arc<ProcessGroup>) -> bool {
        let mut state = self.lock();
        state.process_group = Some(process_group);
        !state.stop_requested
    }

    fn lock(&self) -> MutexGuard<'_, ControlState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Runs `cli` with `args` until it ends, the time-out passes or `control`
/// asks to stop (BR2.2, BR2.4, BR3.2, BR3.3). `on_started` is called once
/// the CLI runs; `on_hints` for every output line that holds an
/// allow-listed URL or a code (BR3.1). Blocks the calling thread.
pub fn run_login(
    cli: &Path,
    args: &[String],
    control: &LoginControl,
    times: LoginTimes,
    on_started: impl FnOnce(),
    mut on_hints: impl FnMut(LoginHints),
) -> SsoLoginOutcome {
    if control.stop_requested() {
        return SsoLoginOutcome::Cancelled;
    }
    let started_at = Instant::now();
    let (mut child, group) = match spawn_cli(cli, args) {
        Ok(started) => started,
        Err(error) => {
            // The kind only: the message could name local paths.
            eprintln!(
                "local-sights: sso login: the AWS CLI could not be started ({:?})",
                error.kind()
            );
            return SsoLoginOutcome::Failed { exit_code: None };
        }
    };
    let group = Arc::new(group);
    if !control.register(Arc::clone(&group)) {
        stop_child(&mut child, &group, times.grace);
        return SsoLoginOutcome::Cancelled;
    }
    let hints = forward_output(&mut child);
    on_started();
    loop {
        match hints.recv_timeout(POLL_INTERVAL) {
            Ok(found) => on_hints(found),
            Err(RecvTimeoutError::Timeout) => {}
            // Both outputs are closed; keep watching the process.
            Err(RecvTimeoutError::Disconnected) => thread::sleep(POLL_INTERVAL),
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                // Stopped from outside (the app's exit): not a failure.
                if control.stop_requested() {
                    return SsoLoginOutcome::Cancelled;
                }
                // Ended by itself: what it may have started is left alone.
                group.release();
                drain_hints(&hints, &mut on_hints);
                return match status.code() {
                    Some(0) => SsoLoginOutcome::Succeeded,
                    code => SsoLoginOutcome::Failed { exit_code: code },
                };
            }
            Ok(None) => {}
            Err(error) => {
                eprintln!(
                    "local-sights: sso login: the AWS CLI could not be watched ({:?})",
                    error.kind()
                );
                stop_child(&mut child, &group, times.grace);
                return SsoLoginOutcome::Failed { exit_code: None };
            }
        }
        if control.stop_requested() {
            stop_child(&mut child, &group, times.grace);
            return SsoLoginOutcome::Cancelled;
        }
        if started_at.elapsed() >= times.timeout {
            stop_child(&mut child, &group, times.grace);
            return SsoLoginOutcome::TimedOut;
        }
    }
}

/// After the CLI ended: hands over the hints its last lines still carry.
/// The readers end when the outputs close; a process the CLI left behind
/// that keeps them open is waited for at most [`DRAIN_LIMIT`].
fn drain_hints(hints: &Receiver<LoginHints>, on_hints: &mut impl FnMut(LoginHints)) {
    let deadline = Instant::now() + DRAIN_LIMIT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match hints.recv_timeout(left) {
            Ok(found) => on_hints(found),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Reads both outputs on their own threads; only hints come back.
fn forward_output(child: &mut Child) -> Receiver<LoginHints> {
    let (sender, receiver) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        spawn_reader(stdout, sender.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_reader(stderr, sender);
    }
    receiver
}

fn spawn_reader(stream: impl Read + Send + 'static, sender: Sender<LoginHints>) {
    let spawned = thread::Builder::new()
        .name("sso-login-output".to_string())
        .spawn(move || forward_hints(stream, &sender));
    if spawned.is_err() {
        // Without the reader the CLI still runs; only its hints are lost.
        eprintln!("local-sights: sso login: the output reader could not be started");
    }
}

/// Splits `stream` into lines of at most [`MAX_LINE_BYTES`] bytes (the
/// rest of a longer line is dropped), sends the hints of each line and
/// drops the line (BR3.1, NFR3). A `\r` before the `\n` (Windows) is
/// whitespace to `extract_login_hints`.
fn forward_hints(mut stream: impl Read, sender: &Sender<LoginHints>) {
    let mut chunk = [0_u8; READ_CHUNK];
    let mut line: Vec<u8> = Vec::with_capacity(256);
    loop {
        let read = match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        for &byte in &chunk[..read] {
            if byte == b'\n' {
                if !send_line_hints(&line, sender) {
                    return;
                }
                line.clear();
            } else if line.len() < MAX_LINE_BYTES {
                line.push(byte);
            }
        }
    }
    send_line_hints(&line, sender);
}

/// Sends the hints of one line, if any; `false` once nobody listens.
fn send_line_hints(line: &[u8], sender: &Sender<LoginHints>) -> bool {
    let hints = extract_login_hints(&String::from_utf8_lossy(line));
    hints.is_empty() || sender.send(hints).is_ok()
}

/// Stops the CLI's process group and reaps the CLI.
fn stop_child(child: &mut Child, group: &ProcessGroup, grace: Duration) {
    group.terminate(grace, || matches!(child.try_wait(), Ok(Some(_)) | Err(_)));
    if let Err(error) = child.wait() {
        eprintln!(
            "local-sights: sso login: the AWS CLI could not be reaped ({:?})",
            error.kind()
        );
    }
}

/// Helpers shared by the Unix and Windows tests: a fake CLI runs one at a
/// time, with short times, and the processes it leaves are looked for.
#[cfg(test)]
mod test_support {
    use std::fs;
    use std::path::Path;
    use std::str::FromStr;
    use std::sync::{Mutex as StdMutex, MutexGuard, PoisonError};
    use std::thread;
    use std::time::{Duration, Instant};

    use local_sights_core::sso_login::{LoginHints, SsoLoginOutcome};

    use super::{LoginControl, LoginTimes, run_login};

    /// One fake CLI runs at a time: writing an executable while another
    /// thread forks can fail its start with "text file busy", and the
    /// timing checks stay steady.
    static SERIAL: StdMutex<()> = StdMutex::new(());

    pub(super) const SHORT: LoginTimes = LoginTimes {
        timeout: Duration::from_secs(20),
        grace: Duration::from_millis(500),
    };

    pub(super) fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn args() -> Vec<String> {
        vec!["sso".to_string(), "login".to_string()]
    }

    /// Runs `cli` and collects what reaches the caller.
    pub(super) fn run(
        cli: &Path,
        control: &LoginControl,
        times: LoginTimes,
    ) -> (SsoLoginOutcome, bool, Vec<LoginHints>) {
        let mut started = false;
        let mut hints = Vec::new();
        let outcome = run_login(
            cli,
            &args(),
            control,
            times,
            || started = true,
            |found| {
                hints.push(found);
            },
        );
        (outcome, started, hints)
    }

    /// Waits up to two seconds for the process `pid` to be gone, as
    /// `is_running` sees it (FR4.4: within two seconds).
    pub(super) fn gone_soon<T: Copy>(pid: T, is_running: impl Fn(T) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if !is_running(pid) {
                return true;
            }
            thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// Waits up to `wait` until the fake CLI wrote its pid files and reads
    /// them, in the order of `names`.
    pub(super) fn read_pids<T: FromStr>(dir: &Path, names: &[&str], wait: Duration) -> Vec<T> {
        let deadline = Instant::now() + wait;
        loop {
            let pids: Option<Vec<T>> = names
                .iter()
                .map(|name| {
                    fs::read_to_string(dir.join(name))
                        .ok()
                        .and_then(|text| text.trim().parse().ok())
                })
                .collect();
            if let Some(pids) = pids {
                return pids;
            }
            assert!(Instant::now() < deadline, "the fake CLI did not start");
            thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Where the AWS CLI is looked for (261015-windows-support FR4.1): pure,
/// so the order is checked without touching the file system.
#[cfg(test)]
mod candidate_tests {
    use std::ffi::OsStr;
    use std::path::PathBuf;

    use super::cli_candidates;

    #[cfg(windows)]
    #[test]
    fn the_official_installer_under_program_files_comes_first() {
        let candidates = cli_candidates(Some(OsStr::new(r"D:\Programs")), None);
        assert_eq!(
            candidates,
            [PathBuf::from(r"D:\Programs\Amazon\AWSCLIV2\aws.exe")]
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_path_folders_follow_in_order_without_empty_or_relative_entries() {
        let candidates = cli_candidates(
            Some(OsStr::new(r"C:\Program Files")),
            Some(OsStr::new(r"C:\a;;C:\b;.;relative\bin")),
        );
        assert_eq!(
            candidates,
            [
                PathBuf::from(r"C:\Program Files\Amazon\AWSCLIV2\aws.exe"),
                PathBuf::from(r"C:\a\aws.exe"),
                PathBuf::from(r"C:\b\aws.exe"),
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn program_files_defaults_to_the_system_drive_when_the_variable_is_missing() {
        let missing = cli_candidates(None, None);
        let empty = cli_candidates(Some(OsStr::new("")), None);
        let expected = [PathBuf::from(r"C:\Program Files\Amazon\AWSCLIV2\aws.exe")];
        assert_eq!(missing, expected);
        assert_eq!(empty, expected);
    }

    #[cfg(unix)]
    #[test]
    fn unix_looks_only_at_homebrew_and_the_official_installer() {
        let expected = [
            PathBuf::from("/opt/homebrew/bin/aws"),
            PathBuf::from("/usr/local/bin/aws"),
        ];
        assert_eq!(cli_candidates(None, None), expected);
        assert_eq!(
            cli_candidates(
                Some(OsStr::new("/Program Files")),
                Some(OsStr::new("/usr/bin:/bin"))
            ),
            expected,
            "PATH is never used on Unix (BR2.1)"
        );
    }
}

/// These tests run fake AWS CLIs: small `/bin/sh` scripts written to a
/// temporary folder. The real AWS CLI, AWS and a browser are never used.
#[cfg(all(test, unix))]
mod unix_tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use local_sights_core::sso_login::{MAX_LINE_BYTES, SsoLoginOutcome};

    use super::test_support::{SHORT, gone_soon, run, serial};
    use super::{LoginControl, LoginTimes, locate_cli, run_login};

    /// Writes an executable `#!/bin/sh` script named `name` into `dir`.
    fn fake_cli(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Whether the process `pid` is still running (a zombie counts as gone:
    /// in a container nobody may reap an orphan).
    fn is_running(pid: libc::pid_t) -> bool {
        let stat = Path::new("/proc").join(pid.to_string()).join("stat");
        if Path::new("/proc").is_dir() {
            return fs::read_to_string(stat).is_ok_and(|text| {
                text.rsplit_once(')')
                    .is_some_and(|(_, rest)| !rest.trim_start().starts_with('Z'))
            });
        }
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    /// Waits until the script wrote its pid files and reads them.
    fn read_pids(dir: &Path, names: &[&str]) -> Vec<libc::pid_t> {
        super::test_support::read_pids(dir, names, Duration::from_secs(5))
    }

    // --- locate_cli (BR2.1) ---

    #[test]
    fn the_second_candidate_is_used_when_only_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        let second = fake_cli(dir.path(), "aws-second", "exit 0");
        let first = dir.path().join("aws-first");
        assert_eq!(
            locate_cli(&[first.as_path(), second.as_path()]),
            Some(second)
        );
    }

    #[test]
    fn nothing_is_found_without_an_executable_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let not_executable = dir.path().join("aws-plain");
        fs::write(&not_executable, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&not_executable, fs::Permissions::from_mode(0o644)).unwrap();
        let missing = dir.path().join("aws-missing");
        let directory = dir.path().join("aws-dir");
        fs::create_dir(&directory).unwrap();
        assert_eq!(
            locate_cli(&[
                not_executable.as_path(),
                missing.as_path(),
                directory.as_path()
            ]),
            None
        );
        assert_eq!(locate_cli::<&Path>(&[]), None);
    }

    // --- run_login (BR2.2-BR3.3) ---

    #[test]
    fn a_successful_cli_hands_over_the_url_and_code_and_succeeds() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = fake_cli(
            dir.path(),
            "aws",
            "echo 'Attempting to automatically open the SSO authorization page'\n\
             echo https://device.sso.us-east-1.amazonaws.com/\n\
             echo 'Then enter the code:' >&2\n\
             echo ABCD-EFGH >&2\n\
             exit 0",
        );
        let (outcome, started, hints) = run(&cli, &LoginControl::default(), SHORT);
        assert_eq!(outcome, SsoLoginOutcome::Succeeded);
        assert!(started);
        let urls: Vec<_> = hints
            .iter()
            .filter_map(|h| h.verification_url.clone())
            .collect();
        let codes: Vec<_> = hints.iter().filter_map(|h| h.user_code.clone()).collect();
        assert_eq!(urls, ["https://device.sso.us-east-1.amazonaws.com/"]);
        assert_eq!(codes, ["ABCD-EFGH"]);
    }

    #[test]
    fn the_arguments_reach_the_cli_as_given_without_a_shell() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("args.txt");
        let cli = fake_cli(
            dir.path(),
            "aws",
            &format!("printf '%s|' \"$@\" > '{}'\nexit 0", out.display()),
        );
        let args = ["sso", "login", "--profile", "dev;echo x"].map(str::to_string);
        let outcome = run_login(&cli, &args, &LoginControl::default(), SHORT, || {}, |_| {});
        assert_eq!(outcome, SsoLoginOutcome::Succeeded);
        assert_eq!(
            fs::read_to_string(out).unwrap(),
            "sso|login|--profile|dev;echo x|"
        );
    }

    #[test]
    fn a_failing_cli_reports_its_exit_code() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = fake_cli(
            dir.path(),
            "aws",
            "echo 'Error loading SSO Token' >&2\nexit 3",
        );
        let (outcome, started, hints) = run(&cli, &LoginControl::default(), SHORT);
        assert_eq!(outcome, SsoLoginOutcome::Failed { exit_code: Some(3) });
        assert!(started);
        assert!(hints.is_empty(), "the error text is never handed over");
    }

    #[test]
    fn a_cli_that_cannot_start_fails_without_an_exit_code() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let (outcome, started, _) = run(
            &dir.path().join("no-such-aws"),
            &LoginControl::default(),
            SHORT,
        );
        assert_eq!(outcome, SsoLoginOutcome::Failed { exit_code: None });
        assert!(!started);
    }

    #[test]
    fn disallowed_urls_and_long_lines_never_reach_the_caller() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let long = "x".repeat(MAX_LINE_BYTES + 10);
        let cli = fake_cli(
            dir.path(),
            "aws",
            &format!(
                "echo https://evil.example.com/device\n\
                 echo http://device.sso.us-east-1.amazonaws.com/\n\
                 printf '%s https://device.sso.us-east-1.amazonaws.com/ ABCD-EFGH\\n' '{long}'\n\
                 exit 0"
            ),
        );
        let (outcome, _, hints) = run(&cli, &LoginControl::default(), SHORT);
        assert_eq!(outcome, SsoLoginOutcome::Succeeded);
        assert!(hints.is_empty(), "{hints:?}");
    }

    #[test]
    fn a_login_that_takes_too_long_times_out_and_leaves_no_process() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = fake_cli(
            dir.path(),
            "aws",
            &format!(
                "echo $$ > '{dir}/cli.pid'\nsleep 600 &\necho $! > '{dir}/child.pid'\nwait",
                dir = dir.path().display()
            ),
        );
        let times = LoginTimes {
            timeout: Duration::from_millis(500),
            grace: Duration::from_millis(500),
        };
        let begun = Instant::now();
        let (outcome, started, _) = run(&cli, &LoginControl::default(), times);
        assert_eq!(outcome, SsoLoginOutcome::TimedOut);
        assert!(started);
        assert!(begun.elapsed() < Duration::from_secs(5));
        for pid in read_pids(dir.path(), &["cli.pid", "child.pid"]) {
            assert!(gone_soon(pid, is_running), "process {pid} is left");
        }
    }

    #[test]
    fn cancelling_stops_the_whole_process_group() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = fake_cli(
            dir.path(),
            "aws",
            &format!(
                "echo $$ > '{dir}/cli.pid'\nsleep 600 &\necho $! > '{dir}/child.pid'\nwait",
                dir = dir.path().display()
            ),
        );
        let control = LoginControl::new();
        let runner_control = Arc::clone(&control);
        let runner = thread::spawn(move || run(&cli, &runner_control, SHORT).0);
        let pids = read_pids(dir.path(), &["cli.pid", "child.pid"]);
        control.request_stop();
        assert_eq!(runner.join().unwrap(), SsoLoginOutcome::Cancelled);
        for pid in pids {
            assert!(gone_soon(pid, is_running), "process {pid} is left");
        }
    }

    #[test]
    fn processes_that_ignore_sigterm_are_killed_after_the_grace_time() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = fake_cli(
            dir.path(),
            "aws",
            &format!(
                "trap '' TERM\necho $$ > '{dir}/cli.pid'\nsleep 600 &\necho $! > '{dir}/child.pid'\n\
                 while true; do wait; done",
                dir = dir.path().display()
            ),
        );
        let control = LoginControl::new();
        let runner_control = Arc::clone(&control);
        let runner = thread::spawn(move || run(&cli, &runner_control, SHORT).0);
        let pids = read_pids(dir.path(), &["cli.pid", "child.pid"]);
        let begun = Instant::now();
        control.request_stop();
        assert_eq!(runner.join().unwrap(), SsoLoginOutcome::Cancelled);
        assert!(
            begun.elapsed() >= SHORT.grace,
            "SIGKILL only after the grace time"
        );
        for pid in pids {
            assert!(gone_soon(pid, is_running), "process {pid} is left");
        }
    }

    #[test]
    fn stopping_for_the_app_exit_stops_the_cli_before_returning() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = fake_cli(
            dir.path(),
            "aws",
            &format!(
                "echo $$ > '{dir}/cli.pid'\nsleep 600 &\necho $! > '{dir}/child.pid'\nwait",
                dir = dir.path().display()
            ),
        );
        let control = LoginControl::new();
        let runner_control = Arc::clone(&control);
        let runner = thread::spawn(move || run(&cli, &runner_control, SHORT).0);
        let pids = read_pids(dir.path(), &["cli.pid", "child.pid"]);
        control.stop_now(Duration::from_millis(500));
        // The child was signalled before stop_now returned.
        assert!(gone_soon(pids[1], is_running), "the CLI's child is left");
        assert_eq!(runner.join().unwrap(), SsoLoginOutcome::Cancelled);
        assert!(gone_soon(pids[0], is_running), "the CLI is left");
    }

    #[test]
    fn a_stop_asked_before_the_start_starts_nothing() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("ran");
        let cli = fake_cli(dir.path(), "aws", &format!("touch '{}'", marker.display()));
        let control = LoginControl::default();
        control.request_stop();
        let (outcome, started, _) = run(&cli, &control, SHORT);
        assert_eq!(outcome, SsoLoginOutcome::Cancelled);
        assert!(!started);
        assert!(!marker.exists());
    }
}

/// These tests run fake AWS CLIs on Windows: `.cmd` batch files written to
/// a temporary folder (261015-windows-support NFR2). `Command` starts a
/// `.cmd` through `cmd.exe`; that is the test tool, not how the app starts
/// the real `aws.exe`. A fake that needs a child starts PowerShell, which
/// writes the pids and sleeps. The real AWS CLI, AWS and a browser are
/// never used.
#[cfg(all(test, windows))]
mod windows_tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use local_sights_core::sso_login::SsoLoginOutcome;

    use super::test_support::{SHORT, gone_soon, run, serial};
    use super::{LoginControl, LoginTimes, locate_cli};

    /// Writes a batch file named `name` into `dir`; `body` is one command
    /// per line.
    fn fake_cli(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        let lines: Vec<&str> = body.lines().collect();
        fs::write(&path, format!("@echo off\r\n{}\r\n", lines.join("\r\n"))).unwrap();
        path
    }

    /// A batch whose child (PowerShell) writes its own pid to `child.pid`,
    /// the batch's `cmd.exe` pid to `cli.pid`, and sleeps for ten minutes.
    fn sleeping_cli(dir: &Path) -> PathBuf {
        let folder = dir.display();
        fake_cli(
            dir,
            "aws.cmd",
            &format!(
                "powershell -NoProfile -NonInteractive -Command \
                 \"Set-Content -LiteralPath '{folder}\\child.pid' -Value $PID -Encoding ascii; \
                 Set-Content -LiteralPath '{folder}\\cli.pid' -Value \
                 (Get-CimInstance Win32_Process -Filter ('ProcessId = ' + $PID)).ParentProcessId \
                 -Encoding ascii; \
                 Start-Sleep -Seconds 600\""
            ),
        )
    }

    /// Whether the process `pid` is still running, as `tasklist` sees it.
    fn is_running(pid: u32) -> bool {
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\""))
    }

    /// Waits until PowerShell wrote the pid files (it takes a few seconds
    /// to start on a CI runner) and reads them.
    fn read_pids(dir: &Path) -> Vec<u32> {
        super::test_support::read_pids(dir, &["cli.pid", "child.pid"], Duration::from_secs(60))
    }

    // --- locate_cli (FR4.1) ---

    #[test]
    fn the_second_exe_candidate_is_used_when_only_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        let second = dir.path().join("aws-second.EXE");
        fs::write(&second, b"not run").unwrap();
        let first = dir.path().join("aws-first.exe");
        assert_eq!(
            locate_cli(&[first.as_path(), second.as_path()]),
            Some(second)
        );
    }

    #[test]
    fn nothing_is_found_without_an_exe_file() {
        let dir = tempfile::tempdir().unwrap();
        let batch = dir.path().join("aws.cmd");
        fs::write(&batch, b"@echo off\r\n").unwrap();
        let missing = dir.path().join("aws-missing.exe");
        let directory = dir.path().join("aws-dir.exe");
        fs::create_dir(&directory).unwrap();
        assert_eq!(
            locate_cli(&[batch.as_path(), missing.as_path(), directory.as_path()]),
            None
        );
        assert_eq!(locate_cli::<&Path>(&[]), None);
    }

    // --- run_login (FR4.2-FR4.4) ---

    #[test]
    fn a_successful_cli_hands_over_the_url_and_code_and_succeeds() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = fake_cli(
            dir.path(),
            "aws.cmd",
            "echo Attempting to automatically open the SSO authorization page\n\
             echo https://device.sso.us-east-1.amazonaws.com/\n\
             >&2 echo Then enter the code:\n\
             >&2 echo ABCD-EFGH\n\
             exit 0",
        );
        let (outcome, started, hints) = run(&cli, &LoginControl::default(), SHORT);
        assert_eq!(outcome, SsoLoginOutcome::Succeeded);
        assert!(started);
        let urls: Vec<_> = hints
            .iter()
            .filter_map(|h| h.verification_url.clone())
            .collect();
        let codes: Vec<_> = hints.iter().filter_map(|h| h.user_code.clone()).collect();
        assert_eq!(urls, ["https://device.sso.us-east-1.amazonaws.com/"]);
        assert_eq!(codes, ["ABCD-EFGH"]);
    }

    #[test]
    fn a_failing_cli_reports_its_exit_code() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = fake_cli(
            dir.path(),
            "aws.cmd",
            ">&2 echo Error loading SSO Token\nexit 3",
        );
        let (outcome, started, hints) = run(&cli, &LoginControl::default(), SHORT);
        assert_eq!(outcome, SsoLoginOutcome::Failed { exit_code: Some(3) });
        assert!(started);
        assert!(hints.is_empty(), "the error text is never handed over");
    }

    #[test]
    fn a_cli_that_cannot_start_fails_without_an_exit_code() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let (outcome, started, _) = run(
            &dir.path().join("no-such-aws.exe"),
            &LoginControl::default(),
            SHORT,
        );
        assert_eq!(outcome, SsoLoginOutcome::Failed { exit_code: None });
        assert!(!started);
    }

    #[test]
    fn a_login_that_takes_too_long_times_out_and_leaves_no_process() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = sleeping_cli(dir.path());
        // Long enough for PowerShell to start and write the pids first.
        let times = LoginTimes {
            timeout: Duration::from_secs(15),
            grace: Duration::from_millis(500),
        };
        let runner = thread::spawn(move || run(&cli, &LoginControl::default(), times));
        let pids = read_pids(dir.path());
        let (outcome, started, _) = runner.join().unwrap();
        assert_eq!(outcome, SsoLoginOutcome::TimedOut);
        assert!(started);
        for pid in pids {
            assert!(gone_soon(pid, is_running), "process {pid} is left");
        }
    }

    #[test]
    fn cancelling_stops_the_whole_job() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = sleeping_cli(dir.path());
        let control = LoginControl::new();
        let runner_control = Arc::clone(&control);
        let runner = thread::spawn(move || run(&cli, &runner_control, SHORT).0);
        let pids = read_pids(dir.path());
        let begun = Instant::now();
        control.request_stop();
        assert_eq!(runner.join().unwrap(), SsoLoginOutcome::Cancelled);
        assert!(
            begun.elapsed() < Duration::from_secs(5),
            "the job is terminated at once"
        );
        for pid in pids {
            assert!(gone_soon(pid, is_running), "process {pid} is left");
        }
    }

    #[test]
    fn stopping_for_the_app_exit_stops_the_cli_before_returning() {
        let _serial = serial();
        let dir = tempfile::tempdir().unwrap();
        let cli = sleeping_cli(dir.path());
        let control = LoginControl::new();
        let runner_control = Arc::clone(&control);
        let runner = thread::spawn(move || run(&cli, &runner_control, SHORT).0);
        let pids = read_pids(dir.path());
        control.stop_now(Duration::from_millis(500));
        // The job was terminated before stop_now returned.
        assert!(gone_soon(pids[1], is_running), "the CLI's child is left");
        assert_eq!(runner.join().unwrap(), SsoLoginOutcome::Cancelled);
        assert!(gone_soon(pids[0], is_running), "the CLI is left");
    }
}
