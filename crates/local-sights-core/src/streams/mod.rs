//! StreamPlanner: which streams of a log group are fetched (U3:BR1.1-BR1.5).
//!
//! Pure logic: the range test (BR1.3), the early stop on an old stream
//! (BR1.2), the end of pages (BR1.1) and the partial result on an error
//! (BR1.5). The page-by-page flow that calls `DescribeLogStreams` lives in
//! [`planner`].

pub mod planner;

use serde::Serialize;

use crate::failure::ApiFailure;
use crate::paging::{PageDecision, decide};
use crate::time_range::TimeRange;

/// Margin before the range start: a stream whose last event is older than
/// `start − 1 hour` stops the listing (BR1.2; `lastEventTimestamp` is not
/// updated immediately, FR4.2).
pub const LISTING_MARGIN_MS: i64 = 3_600_000;

/// One stream of the log group, from `DescribeLogStreams`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogStream {
    /// Stream name.
    pub log_stream_name: String,
    /// Time of the first event, epoch milliseconds, when the API gave it.
    pub first_event_timestamp: Option<i64>,
    /// Time of the last event, epoch milliseconds, when the API gave it.
    pub last_event_timestamp: Option<i64>,
}

/// How the stream listing ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StreamListingStatus {
    /// Every page was read.
    Complete,
    /// Stopped on a stream older than the margin (BR1.2).
    StoppedEarly,
    /// Stopped by an error; holds what was selected so far (BR1.5).
    Partial,
}

/// The streams one fetch reads, and how the listing ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamPlan {
    /// Selected streams, in listing order (newest last event first).
    pub selected_streams: Vec<LogStream>,
    /// Complete, StoppedEarly or Partial.
    pub listing_status: StreamListingStatus,
    /// The failure, only when Partial.
    pub listing_failure: Option<ApiFailure>,
    /// Number of listing pages received.
    pub listed_page_count: u32,
    /// Number of streams seen in those pages, selected or not.
    pub seen_stream_count: u32,
}

/// The oldest last-event time that keeps the listing going.
pub fn listing_cutoff(range: &TimeRange) -> i64 {
    range.start_instant().saturating_sub(LISTING_MARGIN_MS)
}

/// BR1.2: whether this stream stops the listing (its last event is older
/// than the margin). A stream without a last-event time never does.
pub fn stops_listing(stream: &LogStream, range: &TimeRange) -> bool {
    stream
        .last_event_timestamp
        .is_some_and(|last| last < listing_cutoff(range))
}

/// BR1.3: a stream with both times is selected when it overlaps the range
/// (with the margin before the start); a stream missing either time is
/// always selected, so nothing is missed.
pub fn is_selected(stream: &LogStream, range: &TimeRange) -> bool {
    match (stream.first_event_timestamp, stream.last_event_timestamp) {
        (Some(first), Some(last)) => last >= listing_cutoff(range) && first <= range.end_instant(),
        _ => true,
    }
}

/// Selection state while the listing pages arrive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSelection {
    range: TimeRange,
    selected: Vec<LogStream>,
    seen: u32,
    pages: u32,
    sent_token: Option<String>,
    status: Option<StreamListingStatus>,
}

impl StreamSelection {
    /// Nothing listed yet.
    pub fn new(range: TimeRange) -> Self {
        Self {
            range,
            selected: Vec::new(),
            seen: 0,
            pages: 0,
            sent_token: None,
            status: None,
        }
    }

    /// Token to send with the next `DescribeLogStreams` call.
    pub fn token_to_send(&self) -> Option<&str> {
        self.sent_token.as_deref()
    }

    /// Streams seen so far.
    pub fn seen_count(&self) -> u32 {
        self.seen
    }

    /// Streams selected so far.
    pub fn selected_count(&self) -> u32 {
        u32::try_from(self.selected.len()).unwrap_or(u32::MAX)
    }

    /// Adds one page (BR1.1-BR1.3) and decides whether to read the next.
    ///
    /// On the first stream older than the margin, that stream and every
    /// later stream of the page that has a last-event time are skipped,
    /// streams without one are still selected, and the listing stops
    /// (StoppedEarly). Otherwise no token, or the token just sent, ends the
    /// listing (Complete).
    pub fn apply_page(
        &mut self,
        streams: Vec<LogStream>,
        next_token: Option<&str>,
    ) -> PageDecision {
        self.pages = self.pages.saturating_add(1);
        self.seen = self
            .seen
            .saturating_add(u32::try_from(streams.len()).unwrap_or(u32::MAX));
        let mut stopped = false;
        for stream in streams {
            if !stopped && stops_listing(&stream, &self.range) {
                stopped = true;
            }
            let keep = if stopped {
                stream.last_event_timestamp.is_none()
            } else {
                is_selected(&stream, &self.range)
            };
            if keep {
                self.selected.push(stream);
            }
        }
        if stopped {
            self.status = Some(StreamListingStatus::StoppedEarly);
            return PageDecision::Finished;
        }
        let decision = decide(self.sent_token.as_deref(), next_token);
        match decision {
            PageDecision::Continue => self.sent_token = next_token.map(str::to_string),
            PageDecision::Finished => self.status = Some(StreamListingStatus::Complete),
        }
        decision
    }

    /// Ends the listing on an error after the retries (BR1.5).
    pub fn fail(self, failure: ApiFailure) -> StreamPlan {
        self.plan(StreamListingStatus::Partial, Some(failure))
    }

    /// Ends the listing normally (Complete or StoppedEarly).
    pub fn into_plan(self) -> StreamPlan {
        let status = self.status.unwrap_or(StreamListingStatus::Complete);
        self.plan(status, None)
    }

    fn plan(
        self,
        listing_status: StreamListingStatus,
        listing_failure: Option<ApiFailure>,
    ) -> StreamPlan {
        StreamPlan {
            selected_streams: self.selected,
            listing_status,
            listing_failure,
            listed_page_count: self.pages,
            seen_stream_count: self.seen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure::{FailureKind, SafeDetailFields};

    /// 2024-01-02 03:04:05 UTC to 03:05:05.999.
    fn range() -> TimeRange {
        TimeRange::from_seconds(1_704_164_645, 1_704_164_705).unwrap()
    }

    fn start() -> i64 {
        range().start_instant()
    }

    fn end() -> i64 {
        range().end_instant()
    }

    fn stream(name: &str, first: Option<i64>, last: Option<i64>) -> LogStream {
        LogStream {
            log_stream_name: name.to_string(),
            first_event_timestamp: first,
            last_event_timestamp: last,
        }
    }

    fn names(plan: &StreamPlan) -> Vec<&str> {
        plan.selected_streams
            .iter()
            .map(|s| s.log_stream_name.as_str())
            .collect()
    }

    #[test]
    fn overlapping_streams_and_streams_without_times_are_selected() {
        let r = range();
        assert!(is_selected(&stream("in", Some(start()), Some(end())), &r));
        assert!(is_selected(
            &stream("starts-before", Some(start() - 10), Some(start() + 1)),
            &r
        ));
        assert!(!is_selected(
            &stream("starts-after", Some(end() + 1), Some(end() + 5)),
            &r
        ));
        assert!(is_selected(
            &stream("first-is-end", Some(end()), Some(end() + 9)),
            &r
        ));
        assert!(is_selected(&stream("no-first", None, Some(1)), &r));
        assert!(is_selected(&stream("no-last", Some(end() + 99), None), &r));
        assert!(is_selected(&stream("no-times", None, None), &r));
    }

    #[test]
    fn the_margin_boundary_is_one_hour_before_the_start() {
        let r = range();
        assert_eq!(listing_cutoff(&r), start() - 3_600_000);
        let exactly = stream("exactly", Some(0), Some(start() - LISTING_MARGIN_MS));
        assert!(is_selected(&exactly, &r));

        let mut selection = StreamSelection::new(r);
        let decision = selection.apply_page(
            vec![
                exactly,
                stream("older", Some(0), Some(start() - LISTING_MARGIN_MS - 1)),
            ],
            Some("n1"),
        );
        assert_eq!(decision, PageDecision::Finished);
        let plan = selection.into_plan();
        assert_eq!(plan.listing_status, StreamListingStatus::StoppedEarly);
        assert_eq!(names(&plan), vec!["exactly"]);
    }

    #[test]
    fn an_old_stream_stops_the_listing_but_keeps_streams_without_times() {
        let mut selection = StreamSelection::new(range());
        let old = start() - LISTING_MARGIN_MS - 1;
        let decision = selection.apply_page(
            vec![
                stream("a", Some(start()), Some(end())),
                stream("no-times-before", None, None),
                stream("old", Some(0), Some(old)),
                stream("no-times-after", None, None),
                stream("newer-after", Some(start()), Some(end())),
                stream("no-last-after", Some(start()), None),
            ],
            Some("n1"),
        );
        assert_eq!(decision, PageDecision::Finished);
        assert_eq!(selection.seen_count(), 6);
        let plan = selection.into_plan();
        assert_eq!(plan.listing_status, StreamListingStatus::StoppedEarly);
        assert_eq!(
            names(&plan),
            vec!["a", "no-times-before", "no-times-after", "no-last-after"]
        );
        assert_eq!(plan.listed_page_count, 1);
        assert_eq!(plan.seen_stream_count, 6);
        assert_eq!(plan.listing_failure, None);
    }

    #[test]
    fn a_stream_without_a_last_time_never_stops_the_listing() {
        let mut selection = StreamSelection::new(range());
        let decision = selection.apply_page(vec![stream("no-last", Some(0), None)], Some("n1"));
        assert_eq!(decision, PageDecision::Continue);
        assert_eq!(selection.token_to_send(), Some("n1"));
    }

    #[test]
    fn no_token_or_the_same_token_completes_the_listing() {
        let mut selection = StreamSelection::new(range());
        assert_eq!(
            selection.apply_page(vec![stream("a", None, None)], Some("n1")),
            PageDecision::Continue
        );
        assert_eq!(
            selection.apply_page(vec![stream("b", None, None)], Some("n1")),
            PageDecision::Finished
        );
        let plan = selection.into_plan();
        assert_eq!(plan.listing_status, StreamListingStatus::Complete);
        assert_eq!(plan.listed_page_count, 2);

        let mut selection = StreamSelection::new(range());
        assert_eq!(selection.apply_page(vec![], None), PageDecision::Finished);
        let plan = selection.into_plan();
        assert_eq!(plan.listing_status, StreamListingStatus::Complete);
        assert!(plan.selected_streams.is_empty());
    }

    #[test]
    fn an_error_keeps_the_streams_selected_so_far_as_partial() {
        let failure = ApiFailure::new(FailureKind::Throttled, &SafeDetailFields::default());
        let mut selection = StreamSelection::new(range());
        selection.apply_page(
            vec![
                stream("a", Some(start()), Some(end())),
                stream("skipped", Some(end() + 1), Some(end() + 2)),
            ],
            Some("n1"),
        );
        assert_eq!(selection.selected_count(), 1);
        let plan = selection.fail(failure.clone());
        assert_eq!(plan.listing_status, StreamListingStatus::Partial);
        assert_eq!(plan.listing_failure, Some(failure.clone()));
        assert_eq!(names(&plan), vec!["a"]);
        assert_eq!(plan.listed_page_count, 1);

        let plan = StreamSelection::new(range()).fail(failure);
        assert!(plan.selected_streams.is_empty());
        assert_eq!(plan.listed_page_count, 0);
    }

    #[test]
    fn selected_streams_keep_the_listing_order_across_pages() {
        let mut selection = StreamSelection::new(range());
        selection.apply_page(
            vec![stream("z", None, None), stream("b", None, None)],
            Some("n1"),
        );
        selection.apply_page(vec![stream("a", None, None)], Some("n2"));
        assert_eq!(selection.seen_count(), 3);
        selection.apply_page(vec![], None);
        assert_eq!(names(&selection.into_plan()), vec!["z", "b", "a"]);
    }
}
