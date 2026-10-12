//! Running `aws sso login` of the AWS CLI (261011-sso-login).
//!
//! The decisions (where to sign in, the arguments, what of the output may
//! be shown) live in `local_sights_core::sso_login`; this module only finds
//! the CLI and runs it:
//!
//! - [`locate_cli`]: the first candidate that is an executable file, in the
//!   order `/opt/homebrew/bin/aws`, `/usr/local/bin/aws`. `PATH` is not
//!   used: an app started from Finder or the Dock does not get the
//!   terminal's `PATH` (BR2.1).
//! - [`run_login`]: starts the CLI without a shell, with the given
//!   arguments only, standard input closed and the app's environment as is
//!   (BR2.2, BR2.4), in a new process group. Standard output and standard
//!   error are read line by line on their own threads; each line (at most
//!   [`MAX_LINE_BYTES`] bytes of it) is handed to `extract_login_hints` and
//!   dropped: only the allow-listed URL and the code leave this module,
//!   nothing is kept or logged (BR3.1, NFR3).
//! - Cancel, the time-out (five minutes, BR3.2) and the app's exit stop the
//!   whole process group: SIGTERM, then SIGKILL to what is left after the
//!   grace time (BR3.3, NFR4). A CLI that ends by itself is left alone, so
//!   a browser it started is not closed.
//!
//! Unix only (the app is distributed for macOS; CI also builds on Linux).

use std::io::{ErrorKind, Read};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use local_sights_core::sso_login::{
    LoginHints, MAX_LINE_BYTES, SsoLoginOutcome, extract_login_hints,
};

/// Where the AWS CLI is looked for, in this order (BR2.1): Homebrew on
/// Apple Silicon, then the official installer.
pub const CLI_CANDIDATES: [&str; 2] = ["/opt/homebrew/bin/aws", "/usr/local/bin/aws"];
/// How long the approval in the browser is waited for (BR3.2).
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// How long a stopped CLI gets to end after SIGTERM before SIGKILL (BR3.3).
pub const STOP_GRACE: Duration = Duration::from_secs(2);
/// How often the runner looks at the CLI, the stop request and the clock.
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// Bytes read from the CLI at once.
const READ_CHUNK: usize = 1024;
/// How long the last output lines are waited for after the CLI ended.
const DRAIN_LIMIT: Duration = Duration::from_millis(500);

/// The first candidate that is an executable regular file (symbolic links
/// are followed, as Homebrew's `aws` is one), or `None` (BR2.1).
pub fn locate_cli<P: AsRef<Path>>(candidates: &[P]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(AsRef::as_ref)
        .find(|path| is_executable_file(path))
        .map(Path::to_path_buf)
}

fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// How long [`run_login`] waits; the app uses [`LOGIN_TIMEOUT`] and
/// [`STOP_GRACE`], tests use shorter times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginTimes {
    /// The time-out of the whole login (BR3.2).
    pub timeout: Duration,
    /// The time between SIGTERM and SIGKILL (BR3.3).
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
    /// The CLI's process group, once it runs.
    process_group: Option<libc::pid_t>,
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
    /// group right here, waiting at most `grace` before SIGKILL (BR3.3).
    pub fn stop_now(&self, grace: Duration) {
        let process_group = {
            let mut state = self.lock();
            state.stop_requested = true;
            state.process_group
        };
        if let Some(group) = process_group {
            terminate_group(group, grace, || false);
        }
    }

    fn stop_requested(&self) -> bool {
        self.lock().stop_requested
    }

    /// Records the CLI's process group; `false` when a stop was asked
    /// before, so the caller stops it at once. Under one lock with
    /// [`LoginControl::stop_now`], so a stop never misses the CLI.
    fn register(&self, process_group: libc::pid_t) -> bool {
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
    let mut child = match spawn_cli(cli, args) {
        Ok(child) => child,
        Err(error) => {
            // The kind only: the message could name local paths.
            eprintln!(
                "local-sights: sso login: the AWS CLI could not be started ({:?})",
                error.kind()
            );
            return SsoLoginOutcome::Failed { exit_code: None };
        }
    };
    let Some(group) = process_group_of(&child) else {
        eprintln!("local-sights: sso login: the AWS CLI has no usable process id");
        stop_child(&mut child, None, times.grace);
        return SsoLoginOutcome::Failed { exit_code: None };
    };
    if !control.register(group) {
        stop_child(&mut child, Some(group), times.grace);
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
                stop_child(&mut child, Some(group), times.grace);
                return SsoLoginOutcome::Failed { exit_code: None };
            }
        }
        if control.stop_requested() {
            stop_child(&mut child, Some(group), times.grace);
            return SsoLoginOutcome::Cancelled;
        }
        if started_at.elapsed() >= times.timeout {
            stop_child(&mut child, Some(group), times.grace);
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

/// Starts the CLI: no shell, the arguments as given, standard input closed,
/// both outputs piped, the environment inherited, a new process group.
fn spawn_cli(cli: &Path, args: &[String]) -> std::io::Result<Child> {
    Command::new(cli)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
}

/// The CLI's process group: its own pid (`process_group(0)`). `None` for a
/// pid that cannot be a group of its own, which is never signalled.
fn process_group_of(child: &Child) -> Option<libc::pid_t> {
    libc::pid_t::try_from(child.id())
        .ok()
        .filter(|pid| *pid > 1)
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
/// drops the line (BR3.1, NFR3).
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

/// Stops the CLI's process group (or only the CLI without one) and reaps
/// the CLI.
fn stop_child(child: &mut Child, group: Option<libc::pid_t>, grace: Duration) {
    match group {
        Some(group) => terminate_group(group, grace, || {
            matches!(child.try_wait(), Ok(Some(_)) | Err(_))
        }),
        None => {
            if let Err(error) = child.kill() {
                eprintln!(
                    "local-sights: sso login: the AWS CLI could not be stopped ({:?})",
                    error.kind()
                );
            }
        }
    }
    if let Err(error) = child.wait() {
        eprintln!(
            "local-sights: sso login: the AWS CLI could not be reaped ({:?})",
            error.kind()
        );
    }
}

/// SIGTERM to the group, then up to `grace` for it to end (`reap` lets the
/// caller collect the leader meanwhile), then SIGKILL to what is left.
fn terminate_group(group: libc::pid_t, grace: Duration, mut reap: impl FnMut() -> bool) {
    signal_group(group, libc::SIGTERM);
    let deadline = Instant::now() + grace;
    loop {
        reap();
        if !group_exists(group) {
            return;
        }
        if Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    signal_group(group, libc::SIGKILL);
    reap();
}

/// Sends `signal` to every process of `group` (> 1, never this app's own
/// group, since the CLI was given a group of its own).
fn signal_group(group: libc::pid_t, signal: libc::c_int) {
    if group <= 1 {
        return;
    }
    // SAFETY: killpg only sends a signal; `group` is the process group the
    // CLI was started in, and values that would address this app's own
    // group (0) or every process (1, -1) are refused above.
    let result = unsafe { libc::killpg(group, signal) };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            eprintln!("local-sights: sso login: the AWS CLI could not be signalled ({error})");
        }
    }
}

/// Whether any process of `group` still exists.
fn group_exists(group: libc::pid_t) -> bool {
    if group <= 1 {
        return false;
    }
    // SAFETY: signal 0 only checks that the group exists; nothing is sent.
    unsafe { libc::killpg(group, 0) == 0 }
}

/// These tests run fake AWS CLIs: small `/bin/sh` scripts written to a
/// temporary folder. The real AWS CLI, AWS and a browser are never used.
#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex as StdMutex;

    use super::*;

    /// One fake CLI runs at a time: writing an executable while another
    /// thread forks can fail its start with "text file busy", and the
    /// timing checks stay steady.
    static SERIAL: StdMutex<()> = StdMutex::new(());

    const SHORT: LoginTimes = LoginTimes {
        timeout: Duration::from_secs(20),
        grace: Duration::from_millis(500),
    };

    fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Writes an executable `#!/bin/sh` script named `name` into `dir`.
    fn fake_cli(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn args() -> Vec<String> {
        vec!["sso".to_string(), "login".to_string()]
    }

    /// Runs `cli` and collects what reaches the caller.
    fn run(
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

    /// Waits up to two seconds for `pid` to be gone.
    fn gone_soon(pid: libc::pid_t) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if !is_running(pid) {
                return true;
            }
            thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// Waits until the script wrote its pid files and reads them.
    fn read_pids(dir: &Path, names: &[&str]) -> Vec<libc::pid_t> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let pids: Option<Vec<libc::pid_t>> = names
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
            assert!(gone_soon(pid), "process {pid} is left");
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
            assert!(gone_soon(pid), "process {pid} is left");
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
            assert!(gone_soon(pid), "process {pid} is left");
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
        assert!(gone_soon(pids[1]), "the CLI's child is left");
        assert_eq!(runner.join().unwrap(), SsoLoginOutcome::Cancelled);
        assert!(gone_soon(pids[0]), "the CLI is left");
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
