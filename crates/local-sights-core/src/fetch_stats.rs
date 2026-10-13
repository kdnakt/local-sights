//! FetchStats: the counts of one fetch and its one-line summary
//! (261012-parallel-fetch U1:NFR2.2-NFR2.4, NFR8.3, NFR8.6).
//!
//! The FetchCoordinator makes one [`FetchStats`] per fetch and drops it with
//! the fetch. The `GetLogEvents` attempts are counted where they happen, in
//! the stream readers that run side by side inside the fetch task: every
//! reader holds a shared reference and increments atomic counters (NFR
//! design review R-02). `calls` counts every attempt, retries included, and
//! `throttled` the attempts that failed as Throttled; `max_in_flight` is
//! the largest number of streams being read at once.
//!
//! When the summary is written (review R-02): once at the end of every fetch
//! that went to AWS, whatever its status (Completed, CompletedWithFailures,
//! Failed, Aborted), a group with no stream included. A fetch served from
//! the disk cache writes none. `streams` is the number of streams that
//! ended, skipped ones included; `elapsed_ms` runs from the start of the
//! read stage to its end, and is 0 when the read stage was never entered
//! (the listing failed or the fetch was stopped while listing).
//!
//! The line goes to standard error only, never to a file or the network
//! (NFR8.4), and is built from numbers and the fixed [`JobStatus`] word
//! alone: [`FetchSummary`] has no field that could carry a name, a message
//! or a failure detail (NFR8.3, NFR8.6).
//!
//! ```text
//! local-sights: fetch summary streams=180 calls=212 throttled=3 elapsed_ms=18450 max_in_flight=8 status=Completed
//! ```

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::fetch_status::JobStatus;

/// Counts of one fetch, shared by reference with its stream readers.
#[derive(Debug, Default)]
pub struct FetchStats {
    calls: AtomicU64,
    throttled: AtomicU64,
    max_in_flight: AtomicU64,
}

impl FetchStats {
    /// Counts at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// One `GetLogEvents` attempt was made; `throttled` when it failed as
    /// Throttled (NFR2.4).
    pub fn record_attempt(&self, throttled: bool) {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if throttled {
            self.throttled.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// `count` streams are being read now; the largest count is kept.
    pub fn record_in_flight(&self, count: usize) {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        self.max_in_flight.fetch_max(count, Ordering::Relaxed);
    }

    /// `GetLogEvents` attempts so far.
    pub fn calls(&self) -> u64 {
        self.calls.load(Ordering::Relaxed)
    }

    /// Attempts that failed as Throttled so far.
    pub fn throttled(&self) -> u64 {
        self.throttled.load(Ordering::Relaxed)
    }

    /// The most streams read at once so far.
    pub fn max_in_flight(&self) -> u64 {
        self.max_in_flight.load(Ordering::Relaxed)
    }

    /// The summary of a finished fetch with `streams` ended streams, a read
    /// stage of `elapsed_ms` and its final `status`.
    pub fn summary(&self, streams: u64, elapsed_ms: u64, status: JobStatus) -> FetchSummary {
        FetchSummary {
            streams,
            calls: self.calls(),
            throttled: self.throttled(),
            elapsed_ms,
            max_in_flight: self.max_in_flight(),
            status,
        }
    }
}

/// The values of the one-line summary: numbers and the status only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchSummary {
    /// Streams that ended, skipped ones included.
    pub streams: u64,
    /// `GetLogEvents` attempts, retries included.
    pub calls: u64,
    /// Attempts that failed as Throttled.
    pub throttled: u64,
    /// Milliseconds from the start to the end of the read stage.
    pub elapsed_ms: u64,
    /// The most streams read at once.
    pub max_in_flight: u64,
    /// The final status of the fetch.
    pub status: JobStatus,
}

impl FetchSummary {
    /// The line, in its fixed format (NFR2.2).
    pub fn line(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for FetchSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "local-sights: fetch summary streams={} calls={} throttled={} elapsed_ms={} \
             max_in_flight={} status={}",
            self.streams,
            self.calls,
            self.throttled,
            self.elapsed_ms,
            self.max_in_flight,
            self.status.as_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch_status::JobStatus;

    fn summary(status: JobStatus) -> FetchSummary {
        FetchSummary {
            streams: 180,
            calls: 212,
            throttled: 3,
            elapsed_ms: 18_450,
            max_in_flight: 8,
            status,
        }
    }

    #[test]
    fn the_line_has_the_fixed_fields_in_the_fixed_order() {
        assert_eq!(
            summary(JobStatus::Completed).line(),
            "local-sights: fetch summary streams=180 calls=212 throttled=3 \
             elapsed_ms=18450 max_in_flight=8 status=Completed"
        );
        assert_eq!(
            summary(JobStatus::Aborted).to_string(),
            summary(JobStatus::Aborted).line()
        );
    }

    #[test]
    fn the_values_are_only_integers_and_a_status_word() {
        for status in JobStatus::ALL {
            let line = summary(status).line();
            let fields = line
                .strip_prefix("local-sights: fetch summary ")
                .expect("fixed prefix");
            let pairs: Vec<(&str, &str)> = fields
                .split(' ')
                .map(|pair| pair.split_once('=').expect("key=value"))
                .collect();
            let keys: Vec<&str> = pairs.iter().map(|(key, _)| *key).collect();
            assert_eq!(
                keys,
                [
                    "streams",
                    "calls",
                    "throttled",
                    "elapsed_ms",
                    "max_in_flight",
                    "status"
                ]
            );
            for (key, value) in &pairs[..5] {
                assert!(value.chars().all(|c| c.is_ascii_digit()), "{key}={value}");
            }
            assert_eq!(pairs[5].1, status.as_str());
        }
    }

    #[test]
    fn calls_and_throttled_calls_are_counted_from_shared_references() {
        let stats = FetchStats::new();
        let reader = &stats;
        let other_reader = &stats;
        reader.record_attempt(false);
        other_reader.record_attempt(true);
        reader.record_attempt(true);
        assert_eq!((stats.calls(), stats.throttled()), (3, 2));
        let line = stats.summary(2, 10, JobStatus::Completed).line();
        assert!(line.contains(" calls=3 throttled=2 "), "{line}");
    }

    #[test]
    fn max_in_flight_keeps_the_largest_count_seen() {
        let stats = FetchStats::new();
        for count in [1, 4, 8, 3, 0] {
            stats.record_in_flight(count);
        }
        assert_eq!(stats.max_in_flight(), 8);
        assert_eq!(stats.summary(0, 0, JobStatus::Aborted).max_in_flight, 8);
    }

    #[test]
    fn a_fetch_that_read_nothing_reports_zeros() {
        let stats = FetchStats::default();
        assert_eq!(
            stats.summary(0, 0, JobStatus::Failed).line(),
            "local-sights: fetch summary streams=0 calls=0 throttled=0 \
             elapsed_ms=0 max_in_flight=0 status=Failed"
        );
    }
}
