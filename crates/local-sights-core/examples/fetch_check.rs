//! fetch_check: developer-only check of local-sights against real AWS.
//!
//! Uses the same shared validation (U1:BR1.8) and FetchCoordinator as the
//! desktop app, with the AWS SDK gateway, and fetches the whole log group:
//! every stream that matters for the range (U3:BR6.9). Run it on your own
//! machine with your own AWS credentials (team.md Walking Skeleton). It is
//! never run by `cargo test` or CI (U1:BR7.2) and is not part of the
//! distributed app.
//!
//! ```text
//! cargo run -p local-sights-core --example fetch_check -- \
//!   [--profile NAME] --log-group NAME \
//!   --start "yyyy-mm-dd hh:mm:ss" --end "yyyy-mm-dd hh:mm:ss"
//! ```
//!
//! Connection (U2, review R-11): without `--profile` the fetch uses the
//! "default settings" profile (`ProfileSelector::SdkDefault`); with
//! `--profile NAME` it uses that named profile (`ProfileSelector::Named`).
//! There is no region argument: the profile's default region is used
//! (U2:BR2.8, U1:BR1.6).
//!
//! Standard output: one event per line, in time order (U3:BR4.1),
//! `UTC time<TAB>stream name<TAB>message on one line`.
//! Standard error: the event, stream and failure counts, then every failed
//! stream (or the listing failure) with its kind and safe detail.
//! Exit codes: 0 success, 1 something failed, 2 invalid arguments.

use std::io::{self, BufWriter, Write};
use std::process::ExitCode;
use std::sync::{Mutex, PoisonError};

use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink, JobStatus, run_fetch};
use local_sights_core::event::LogEvent;
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::gateway::aws::AwsCloudWatchLogsGateway;
use local_sights_core::request::{FetchInput, ValidationError, validate_fetch_input};
use local_sights_core::retry::{AbortSignal, RandomJitter, Retrier, RetryPolicy};
use local_sights_core::streams::StreamListingStatus;
use local_sights_core::streams::planner::ListingProgress;
use local_sights_core::time_range::format_utc_millis;
use local_sights_core::timeline::EventTimeline;

const USAGE: &str = "usage: fetch_check [--profile NAME] --log-group NAME \
--start \"yyyy-mm-dd hh:mm:ss\" --end \"yyyy-mm-dd hh:mm:ss\"   (times in UTC)";

const EXIT_FETCH_FAILED: u8 = 1;
const EXIT_USAGE: u8 = 2;

#[tokio::main]
async fn main() -> ExitCode {
    let input = match parse_args(std::env::args().skip(1)) {
        Ok(input) => input,
        Err(message) => return usage_error(&[message]),
    };
    // The shared validation maps a missing or blank --profile to SdkDefault
    // and a name to Named, and leaves the region unset (R-11).
    let validated = match validate_fetch_input(&input) {
        Ok(validated) => validated,
        Err(errors) => {
            let reasons: Vec<String> = errors.iter().map(|e| describe(*e).to_string()).collect();
            return usage_error(&reasons);
        }
    };

    let gateway = AwsCloudWatchLogsGateway::new();
    let timeline = Mutex::new(EventTimeline::new());
    let policy = RetryPolicy::default();
    let jitter = RandomJitter;
    let abort = AbortSignal::never();
    let retrier = Retrier::new(&policy, &jitter, &abort);
    let mut sink = ProgressSink;
    let job = run_fetch(&gateway, &validated, &timeline, &retrier, &mut sink).await;

    let timeline = timeline.lock().unwrap_or_else(PoisonError::into_inner);
    if let Err(error) = write_events(timeline.events()) {
        eprintln!("error: could not write to standard output: {error}");
        return ExitCode::from(EXIT_FETCH_FAILED);
    }
    report(&job)
}

/// Prints the counts and the failures to standard error and chooses the
/// exit code. Only safe details are printed (U1:BR4.3).
fn report(job: &FetchJob) -> ExitCode {
    eprintln!(
        "events: {}  streams: {}  failed streams: {}",
        job.event_count,
        job.planned_stream_count,
        job.failed_streams.len()
    );
    if job.listing_status == Some(StreamListingStatus::Partial) {
        match &job.listing_failure {
            Some(failure) => eprintln!(
                "error: stream listing stopped part way: {} ({})",
                failure.kind().as_str(),
                failure.safe_detail()
            ),
            None => eprintln!("error: stream listing stopped part way"),
        }
    }
    for failed in &job.failed_streams {
        eprintln!(
            "error: stream {}: {} ({})",
            failed.log_stream_name,
            failed.failure.kind().as_str(),
            failed.failure.safe_detail()
        );
    }
    if job.status == JobStatus::Completed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_FETCH_FAILED)
    }
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
        ValidationError::StartFormat => "--start must be a valid yyyy-mm-dd hh:mm:ss (UTC)",
        ValidationError::EndFormat => "--end must be a valid yyyy-mm-dd hh:mm:ss (UTC)",
        ValidationError::RangeOrder => "--start must be before --end",
    }
}

/// Writes every event, in timeline order, one per line.
fn write_events(events: &[LogEvent]) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    for event in events {
        let message = event
            .message
            .replace("\r\n", " ")
            .replace(['\r', '\n'], " ");
        writeln!(
            out,
            "{}\t{}\t{message}",
            format_utc_millis(event.timestamp),
            event.log_stream_name
        )?;
    }
    out.flush()
}

/// Shows that the check is working, on standard error; the events are
/// written in time order once every stream has been read.
struct ProgressSink;

impl FetchSink for ProgressSink {
    fn on_started(&mut self, _job: &FetchJob, _timeline_version: u64) {
        eprintln!("listing streams...");
    }

    fn on_listing_progress(&mut self, _job_id: u64, _progress: ListingProgress) {}

    fn on_planned(&mut self, job: &FetchJob) {
        eprintln!("fetching {} streams...", job.planned_stream_count);
    }

    fn on_batch(&mut self, _job_id: u64, _progress: BatchProgress) {}

    fn on_stream_finished(&mut self, _job: &FetchJob, _outcome: &StreamFetchOutcome) {}

    fn on_finished(&mut self, _job: &FetchJob) {}
}
