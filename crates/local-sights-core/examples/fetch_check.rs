//! fetch_check: developer-only check of the walking skeleton against real AWS.
//!
//! Uses the same shared validation (BR1.8) and FetchCoordinator as the
//! desktop app, with the AWS SDK gateway. Run it on your own machine with
//! your own AWS credentials (team.md Walking Skeleton). It is never run by
//! `cargo test` or CI (BR7.2) and is not part of the distributed app.
//!
//! ```text
//! cargo run -p local-sights-core --example fetch_check -- \
//!   [--profile NAME] --log-group NAME --stream NAME \
//!   --start "yyyy-mm-dd hh:mm:ss" --end "yyyy-mm-dd hh:mm:ss"
//! ```
//!
//! Standard output: one event per line, `UTC time<TAB>message on one line`.
//! Standard error: the event and page counts, or the failure kind and its
//! safe detail. Exit codes: 0 success, 1 fetch failed, 2 invalid arguments.

use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use local_sights_core::coordinator::{FetchJob, FetchSink, JobStatus, run_fetch};
use local_sights_core::event::LogEvent;
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::request::{FetchInput, ValidationError, validate_fetch_input};
use local_sights_core::time_range::format_utc_millis;
use local_sights_core::timeline::EventTimeline;

const USAGE: &str = "usage: fetch_check [--profile NAME] --log-group NAME --stream NAME \
--start \"yyyy-mm-dd hh:mm:ss\" --end \"yyyy-mm-dd hh:mm:ss\"   (times in UTC)";

const EXIT_FETCH_FAILED: u8 = 1;
const EXIT_USAGE: u8 = 2;

#[tokio::main]
async fn main() -> ExitCode {
    let input = match parse_args(std::env::args().skip(1)) {
        Ok(input) => input,
        Err(message) => return usage_error(&[message]),
    };
    let validated = match validate_fetch_input(&input) {
        Ok(validated) => validated,
        Err(errors) => {
            let reasons: Vec<String> = errors.iter().map(|e| describe(*e).to_string()).collect();
            return usage_error(&reasons);
        }
    };

    let gateway = AwsCloudWatchLogsGateway::new();
    let mut timeline = EventTimeline::new();
    let mut sink = StdoutSink::new();
    let job = run_fetch(&gateway, &validated, &mut timeline, &mut sink).await;

    if let Err(error) = sink.finish() {
        eprintln!("error: could not write to standard output: {error}");
        return ExitCode::from(EXIT_FETCH_FAILED);
    }
    eprintln!("events: {}  pages: {}", job.event_count, job.page_count());
    if job.status == JobStatus::Failed {
        match job.failure() {
            Some(failure) => eprintln!(
                "error: {} ({})",
                failure.kind().as_str(),
                failure.safe_detail()
            ),
            None => eprintln!("error: the fetch failed"),
        }
        return ExitCode::from(EXIT_FETCH_FAILED);
    }
    ExitCode::SUCCESS
}

fn usage_error(reasons: &[String]) -> ExitCode {
    eprintln!("{USAGE}");
    for reason in reasons {
        eprintln!("  - {reason}");
    }
    ExitCode::from(EXIT_USAGE)
}

/// Parses `--name value` pairs into the raw input.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<FetchInput, String> {
    let mut input = FetchInput::default();
    while let Some(flag) = args.next() {
        let slot = match flag.as_str() {
            "--profile" => &mut input.profile_name,
            "--log-group" => &mut input.log_group_name,
            "--stream" => &mut input.log_stream_name,
            "--start" => &mut input.start_text,
            "--end" => &mut input.end_text,
            "-h" | "--help" => return Err("help requested".to_string()),
            other => return Err(format!("unknown argument: {other}")),
        };
        *slot = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
    }
    Ok(input)
}

/// English description of a validation reason for the terminal.
fn describe(error: ValidationError) -> &'static str {
    match error {
        ValidationError::LogGroupRequired => "--log-group is required",
        ValidationError::LogGroupTooLong => "--log-group must be 512 characters or fewer",
        ValidationError::LogStreamRequired => "--stream is required",
        ValidationError::LogStreamTooLong => "--stream must be 512 characters or fewer",
        ValidationError::StartFormat => "--start must be a valid yyyy-mm-dd hh:mm:ss (UTC)",
        ValidationError::EndFormat => "--end must be a valid yyyy-mm-dd hh:mm:ss (UTC)",
        ValidationError::RangeOrder => "--start must be before --end",
    }
}

/// Prints each delivered page to standard output as soon as it arrives.
struct StdoutSink {
    out: BufWriter<io::Stdout>,
    write_error: Option<io::Error>,
}

impl StdoutSink {
    fn new() -> Self {
        Self {
            out: BufWriter::new(io::stdout()),
            write_error: None,
        }
    }

    fn write_events(&mut self, events: &[LogEvent]) -> io::Result<()> {
        for event in events {
            let message = event
                .message
                .replace("\r\n", " ")
                .replace(['\r', '\n'], " ");
            writeln!(
                self.out,
                "{}\t{message}",
                format_utc_millis(event.timestamp)
            )?;
        }
        self.out.flush()
    }

    /// Reports the first write error, if any.
    fn finish(mut self) -> io::Result<()> {
        match self.write_error.take() {
            Some(error) => Err(error),
            None => self.out.flush(),
        }
    }
}

impl FetchSink for StdoutSink {
    fn on_started(&mut self, _job: &FetchJob) {}

    fn on_batch(&mut self, _job_id: &str, events: &[LogEvent], _total: u64) {
        if self.write_error.is_none() {
            if let Err(error) = self.write_events(events) {
                self.write_error = Some(error);
            }
        }
    }

    fn on_finished(&mut self, _job: &FetchJob) {}
}
