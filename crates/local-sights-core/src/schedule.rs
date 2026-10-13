//! StreamSchedule: which stream to read next, at most 8 at a time
//! (261012-parallel-fetch U1:BR1.7, BR1.8, BR1.13, BR1.19).
//!
//! The schedule is pure bookkeeping: it never calls AWS or waits. The
//! FetchCoordinator asks it for the next stream whenever a place is free
//! ([`StreamSchedule::start_next`]) and tells it how each stream ended
//! ([`StreamSchedule::finish`]), in the order the streams end.
//!
//! Since U3 (u3-give-up-rule, ADR-007, BR3.6) the schedule alone owns the
//! give-up rule: the count of streams that used up their retries in a row,
//! the limit ([`GIVE_UP_LIMIT`]) and how each end counts. The RetryPolicy
//! only retries one request, and the EventFetcher only reports how its
//! stream ended ([`StreamFetchEnd`]), which [`StreamSchedule::finish`] takes
//! as it is.
//!
//! The count follows the finishing order (BR3.1): a stream that used up its
//! retries adds one, a success or a failure that is not retried resets it to
//! 0 (BR3.4), and a stream stopped by the close leaves it alone (BR3.5).
//! When it reaches the limit, every stream not started yet is failed without
//! calling the API (Skipped), in the selected order, with a copy of the
//! failure of the stream that reached the limit (BR3.2, NFR3.7, NFR8.9), and
//! a [`GiveUp`] is recorded. Streams being read are read to their end and
//! recorded as they end, but after the give-up the count no longer moves
//! and there is no second give-up (BR3.3).

use crate::failure::ApiFailure;
use crate::fetcher::StreamFetchEnd;

/// Streams read at the same time (U1:BR1.7, NFR5.3). Fixed: neither the
/// settings nor the screen change it.
pub const MAX_IN_FLIGHT: usize = 8;

/// Streams that use up their retries in a row, in finishing order, before
/// the streams not started yet are skipped (U3:BR3.2, BR3.6).
pub const GIVE_UP_LIMIT: u32 = 3;

/// Where one selected stream is (entities.md StreamSlot.state).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    /// Not started yet.
    Unstarted,
    /// Being read.
    InFlight,
    /// Read to its last page.
    Succeeded,
    /// Failed, skipped or stopped.
    Failed,
}

/// How one selected stream ended (entities.md StreamSlot.end).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotEnd {
    /// Read to its last page.
    Completed,
    /// Failed after using up its retries.
    FailedExhausted,
    /// Failed without using up its retries (e.g. AccessDenied).
    FailedNotExhausted,
    /// Stopped by the stop signal (U1: only when the window closes).
    Aborted,
    /// Failed without calling the API, because too many streams in a row
    /// used up their retries (U3:BR3.2).
    Skipped,
}

impl SlotEnd {
    /// How the EventFetcher's end of a stream is recorded (U3 review R-02).
    fn of(end: &StreamFetchEnd) -> Self {
        match end {
            StreamFetchEnd::Completed => SlotEnd::Completed,
            StreamFetchEnd::Failed {
                exhausted: true, ..
            } => SlotEnd::FailedExhausted,
            StreamFetchEnd::Failed {
                exhausted: false, ..
            } => SlotEnd::FailedNotExhausted,
            StreamFetchEnd::Aborted => SlotEnd::Aborted,
        }
    }

    /// The count after a stream ended this way, from `consecutive`
    /// (BR3.1, BR3.4, BR3.5): one more when it used up its retries, 0 after
    /// a success or a failure that is not retried, unchanged when stopped
    /// or skipped.
    fn count_after(self, consecutive: u32) -> u32 {
        match self {
            SlotEnd::FailedExhausted => consecutive.saturating_add(1),
            SlotEnd::Completed | SlotEnd::FailedNotExhausted => 0,
            SlotEnd::Aborted | SlotEnd::Skipped => consecutive,
        }
    }
}

/// Counts streams that used up their retries in a row, in finishing order
/// (BR3.1). Moved here from the RetryPolicy (ADR-007, BR3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExhaustionStreak {
    consecutive: u32,
    limit: u32,
}

impl ExhaustionStreak {
    /// A streak that trips after `limit` exhausted streams in a row; a limit
    /// below 1 is taken as 1.
    fn new(limit: u32) -> Self {
        Self {
            consecutive: 0,
            limit: limit.max(1),
        }
    }

    /// Records how a stream ended (see [`SlotEnd::count_after`]).
    fn record(&mut self, end: SlotEnd) {
        self.consecutive = end.count_after(self.consecutive);
    }

    /// Number of exhausted streams in a row so far.
    fn consecutive(&self) -> u32 {
        self.consecutive
    }

    /// Whether the streams not started yet must be skipped.
    fn limit_reached(&self) -> bool {
        self.consecutive >= self.limit
    }
}

/// One selected stream and how far it got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSlot {
    /// Stream name.
    pub log_stream_name: String,
    /// Position in the selection (newest last event first), from 0.
    pub order: usize,
    /// Where the stream is.
    pub state: SlotState,
    /// How it ended, once Succeeded or Failed.
    pub end: Option<SlotEnd>,
}

/// A stream failed without calling the API (U3:BR3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedStream {
    /// Stream name.
    pub log_stream_name: String,
    /// A copy of the failure of the stream that reached the limit.
    pub failure: ApiFailure,
}

/// The give-up, recorded once per fetch (U3:BR3.2, BR3.3, review R-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GiveUp {
    /// The stream whose exhausted end reached the limit.
    pub triggered_by: String,
    /// Its failure (safe detail only), copied to every skipped stream.
    pub failure: ApiFailure,
    /// Streams skipped then; 0 when every stream had already started.
    pub skipped_count: usize,
}

/// How the read stage ended so far (entities.md ScheduleOutcome). The
/// fetch job's own status is decided by the FetchCoordinator (U1:BR1.15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleEndState {
    /// No stream failed.
    Completed,
    /// At least one stream failed or was skipped.
    CompletedWithFailures,
}

/// Summary of the read stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleOutcome {
    /// Completed or CompletedWithFailures.
    pub end_state: ScheduleEndState,
    /// Streams that used up their retries in a row, in finishing order;
    /// frozen once given up (BR3.3).
    pub exhaustion_streak: u32,
    /// Streams being read or not started yet.
    pub unfinished_streams: usize,
    /// The give-up, or `None` when the fetch did not give up.
    pub give_up: Option<GiveUp>,
}

/// The selected streams of one fetch and which of them to read next.
#[derive(Debug, Clone)]
pub struct StreamSchedule {
    slots: Vec<StreamSlot>,
    /// Index of the first stream that may still be unstarted.
    next_unstarted: usize,
    in_flight: usize,
    finished: usize,
    skipped: usize,
    failed: usize,
    streak: ExhaustionStreak,
    give_up: Option<GiveUp>,
}

impl StreamSchedule {
    /// A schedule of `names`, in the selected order (U1:BR1.8); the streams
    /// not started yet are skipped after [`GIVE_UP_LIMIT`] exhausted streams
    /// in a row (U3:BR3.2).
    pub fn new(names: Vec<String>) -> Self {
        Self::with_limit(names, GIVE_UP_LIMIT)
    }

    /// [`StreamSchedule::new`] with another limit; a limit below 1 is taken
    /// as 1 (U3 security-design.md).
    pub fn with_limit(names: Vec<String>, limit: u32) -> Self {
        let slots = names
            .into_iter()
            .enumerate()
            .map(|(order, log_stream_name)| StreamSlot {
                log_stream_name,
                order,
                state: SlotState::Unstarted,
                end: None,
            })
            .collect();
        Self {
            slots,
            next_unstarted: 0,
            in_flight: 0,
            finished: 0,
            skipped: 0,
            failed: 0,
            streak: ExhaustionStreak::new(limit),
            give_up: None,
        }
    }

    /// Starts the first unstarted stream when fewer than [`MAX_IN_FLIGHT`]
    /// are being read (U1:BR1.7, BR1.8), returning its order and name.
    pub fn start_next(&mut self) -> Option<(usize, &str)> {
        if self.in_flight >= MAX_IN_FLIGHT {
            return None;
        }
        let order = self.next_unstarted;
        let slot = self.slots.get_mut(order)?;
        if slot.state != SlotState::Unstarted {
            return None;
        }
        slot.state = SlotState::InFlight;
        self.next_unstarted += 1;
        self.in_flight += 1;
        Some((order, slot.log_stream_name.as_str()))
    }

    /// Records how the stream at `order` ended, as the EventFetcher reported
    /// it, in finishing order (U3:BR3.1-BR3.5, U1:BR1.19). Until the fetch
    /// gives up, the end moves the count; when an exhausted end brings it to
    /// the limit, every unstarted stream becomes Skipped and is returned, in
    /// the selected order, with a copy of this stream's failure, and the
    /// [`GiveUp`] is recorded. After that the count no longer moves. A
    /// stream that is not being read is left as it is.
    pub fn finish(&mut self, order: usize, end: StreamFetchEnd) -> Vec<SkippedStream> {
        let Some(slot) = self.slots.get_mut(order) else {
            return Vec::new();
        };
        if slot.state != SlotState::InFlight {
            return Vec::new();
        }
        let slot_end = SlotEnd::of(&end);
        slot.state = if slot_end == SlotEnd::Completed {
            SlotState::Succeeded
        } else {
            SlotState::Failed
        };
        slot.end = Some(slot_end);
        let triggered_by = slot.log_stream_name.clone();
        self.in_flight -= 1;
        self.finished += 1;
        if slot_end != SlotEnd::Completed {
            self.failed += 1;
        }
        if self.give_up.is_some() {
            // BR3.3: given up once; the count stays as it was.
            return Vec::new();
        }
        self.streak.record(slot_end);
        match end {
            StreamFetchEnd::Failed {
                exhausted: true,
                failure,
            } if self.streak.limit_reached() => self.give_up(triggered_by, failure),
            _ => Vec::new(),
        }
    }

    /// BR3.2: fails every unstarted stream without calling, with a copy of
    /// `failure`, and records the give-up.
    fn give_up(&mut self, triggered_by: String, failure: ApiFailure) -> Vec<SkippedStream> {
        let mut skipped = Vec::new();
        for slot in &mut self.slots[self.next_unstarted..] {
            slot.state = SlotState::Failed;
            slot.end = Some(SlotEnd::Skipped);
            skipped.push(SkippedStream {
                log_stream_name: slot.log_stream_name.clone(),
                failure: failure.clone(),
            });
        }
        self.next_unstarted = self.slots.len();
        self.finished += skipped.len();
        self.skipped += skipped.len();
        self.failed += skipped.len();
        self.give_up = Some(GiveUp {
            triggered_by,
            failure,
            skipped_count: skipped.len(),
        });
        skipped
    }

    /// Every selected stream, in the selected order.
    pub fn slots(&self) -> &[StreamSlot] {
        &self.slots
    }

    /// Streams being read.
    pub fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// Streams that ended, skipped ones included (U1:BR1.19).
    pub fn finished_count(&self) -> usize {
        self.finished
    }

    /// Streams failed without calling the API.
    pub fn skipped_count(&self) -> usize {
        self.skipped
    }

    /// Whether every stream ended.
    pub fn is_done(&self) -> bool {
        self.finished == self.slots.len()
    }

    /// The read stage so far.
    pub fn outcome(&self) -> ScheduleOutcome {
        ScheduleOutcome {
            end_state: if self.failed == 0 {
                ScheduleEndState::Completed
            } else {
                ScheduleEndState::CompletedWithFailures
            },
            exhaustion_streak: self.streak.consecutive(),
            unfinished_streams: self.slots.len() - self.finished,
            give_up: self.give_up.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure::{ApiFailure, FailureKind, SafeDetailFields};
    use crate::fetcher::StreamFetchEnd;

    fn names(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("s{i:02}")).collect()
    }

    fn failure(kind: FailureKind, stream: &str) -> ApiFailure {
        ApiFailure::new(
            kind,
            &SafeDetailFields {
                log_stream_name: Some(stream.to_string()),
                ..SafeDetailFields::default()
            },
        )
    }

    /// A stream that used up its retries with `failure`.
    fn exhausted_with(failure: &ApiFailure) -> StreamFetchEnd {
        StreamFetchEnd::Failed {
            exhausted: true,
            failure: failure.clone(),
        }
    }

    /// A stream that used up its retries while throttled.
    fn exhausted(stream: &str) -> StreamFetchEnd {
        exhausted_with(&failure(FailureKind::Throttled, stream))
    }

    /// A stream that failed without using up its retries (not retried).
    fn denied(stream: &str) -> StreamFetchEnd {
        StreamFetchEnd::Failed {
            exhausted: false,
            failure: failure(FailureKind::AccessDenied, stream),
        }
    }

    fn start_all(schedule: &mut StreamSchedule) -> Vec<usize> {
        std::iter::from_fn(|| schedule.start_next().map(|(order, _)| order)).collect()
    }

    fn skipped_names(skipped: &[SkippedStream]) -> Vec<&str> {
        skipped
            .iter()
            .map(|stream| stream.log_stream_name.as_str())
            .collect()
    }

    #[test]
    fn next_hands_out_the_streams_in_the_selected_order() {
        let mut schedule = StreamSchedule::new(names(3));
        let started: Vec<(usize, String)> = std::iter::from_fn(|| {
            schedule
                .start_next()
                .map(|(order, name)| (order, name.to_string()))
        })
        .collect();
        assert_eq!(
            started,
            vec![
                (0, "s00".to_string()),
                (1, "s01".to_string()),
                (2, "s02".to_string())
            ]
        );
        assert_eq!(schedule.in_flight(), 3);
        assert!(
            schedule
                .slots()
                .iter()
                .all(|slot| slot.state == SlotState::InFlight)
        );
    }

    #[test]
    fn no_more_than_eight_streams_are_read_at_once() {
        let mut schedule = StreamSchedule::new(names(10));
        assert_eq!(start_all(&mut schedule), (0..8).collect::<Vec<_>>());
        assert_eq!(MAX_IN_FLIGHT, 8);
        assert_eq!(schedule.in_flight(), 8);
        assert!(schedule.start_next().is_none(), "the ninth waits");
        assert_eq!(schedule.slots()[8].state, SlotState::Unstarted);
    }

    #[test]
    fn a_finished_stream_frees_a_place_for_the_next_one() {
        let mut schedule = StreamSchedule::new(names(10));
        start_all(&mut schedule);
        let skipped = schedule.finish(5, StreamFetchEnd::Completed);
        assert!(skipped.is_empty());
        assert_eq!(schedule.slots()[5].state, SlotState::Succeeded);
        assert_eq!(schedule.slots()[5].end, Some(SlotEnd::Completed));
        assert_eq!(schedule.start_next().map(|(order, _)| order), Some(8));
        assert!(schedule.start_next().is_none());
        schedule.finish(0, denied("s00"));
        assert_eq!(schedule.slots()[0].state, SlotState::Failed);
        assert_eq!(schedule.start_next().map(|(order, _)| order), Some(9));
        assert!(!schedule.is_done());
    }

    #[test]
    fn no_stream_means_nothing_to_read_and_done_at_once() {
        let mut schedule = StreamSchedule::new(Vec::new());
        assert!(schedule.start_next().is_none());
        assert!(schedule.is_done());
        assert_eq!(
            schedule.outcome(),
            ScheduleOutcome {
                end_state: ScheduleEndState::Completed,
                exhaustion_streak: 0,
                unfinished_streams: 0,
                give_up: None,
            }
        );
    }

    #[test]
    fn three_exhausted_in_finishing_order_skip_every_unstarted_stream() {
        let mut schedule = StreamSchedule::new(names(11));
        start_all(&mut schedule);
        // Finishing order 6, 2, 7 (not the reading order).
        assert!(
            schedule
                .finish(6, exhausted_with(&failure(FailureKind::Throttled, "s06")))
                .is_empty()
        );
        assert_eq!(schedule.start_next().map(|(order, _)| order), Some(8));
        assert!(
            schedule
                .finish(2, exhausted_with(&failure(FailureKind::Network, "s02")))
                .is_empty()
        );
        assert_eq!(schedule.start_next().map(|(order, _)| order), Some(9));
        let last = failure(FailureKind::Throttled, "s07");
        let skipped = schedule.finish(7, exhausted_with(&last));
        assert_eq!(
            skipped,
            vec![SkippedStream {
                log_stream_name: "s10".to_string(),
                failure: last.clone(),
            }],
            "the last exhausted stream's failure is copied"
        );
        assert_eq!(schedule.slots()[10].state, SlotState::Failed);
        assert_eq!(schedule.slots()[10].end, Some(SlotEnd::Skipped));
        assert!(schedule.start_next().is_none(), "nothing more is started");
        assert_eq!(schedule.outcome().exhaustion_streak, 3);
    }

    #[test]
    fn a_success_or_another_failure_resets_the_count() {
        let mut schedule = StreamSchedule::new(names(9));
        start_all(&mut schedule);
        let exhausted = failure(FailureKind::Throttled, "x");
        schedule.finish(0, exhausted_with(&exhausted));
        schedule.finish(1, exhausted_with(&exhausted));
        schedule.finish(2, StreamFetchEnd::Completed);
        assert_eq!(schedule.outcome().exhaustion_streak, 0);
        schedule.finish(3, exhausted_with(&exhausted));
        schedule.finish(4, exhausted_with(&exhausted));
        schedule.finish(5, denied("s05"));
        assert_eq!(schedule.outcome().exhaustion_streak, 0);
        let skipped = schedule.finish(6, exhausted_with(&exhausted));
        assert!(skipped.is_empty());
        assert_eq!(schedule.outcome().exhaustion_streak, 1);
        assert!(schedule.start_next().is_some(), "the ninth still starts");
    }

    #[test]
    fn finished_and_skipped_streams_are_counted() {
        let mut schedule = StreamSchedule::new(names(12));
        start_all(&mut schedule);
        assert_eq!(
            (schedule.finished_count(), schedule.skipped_count()),
            (0, 0)
        );
        let exhausted = failure(FailureKind::Throttled, "x");
        schedule.finish(0, exhausted_with(&exhausted));
        schedule.finish(1, exhausted_with(&exhausted));
        // Two places freed but not refilled yet: s08 and s09 stay unstarted.
        let skipped = schedule.finish(2, exhausted_with(&exhausted));
        assert_eq!(skipped.len(), 4, "s08 to s11");
        assert_eq!(schedule.skipped_count(), 4);
        assert_eq!(schedule.finished_count(), 7, "three read and four skipped");
        assert_eq!(
            schedule.outcome(),
            ScheduleOutcome {
                end_state: ScheduleEndState::CompletedWithFailures,
                exhaustion_streak: 3,
                unfinished_streams: 5,
                give_up: Some(GiveUp {
                    triggered_by: "s02".to_string(),
                    failure: exhausted.clone(),
                    skipped_count: 4,
                }),
            }
        );
        for order in 3..8 {
            schedule.finish(order, StreamFetchEnd::Completed);
        }
        assert_eq!(schedule.finished_count(), 12);
        assert!(schedule.is_done());
        assert_eq!(schedule.outcome().unfinished_streams, 0);
    }

    #[test]
    fn streams_being_read_are_never_skipped() {
        let mut schedule = StreamSchedule::new(names(10));
        start_all(&mut schedule);
        let exhausted = failure(FailureKind::Throttled, "x");
        for order in 0..3 {
            schedule.finish(order, exhausted_with(&exhausted));
        }
        let in_flight: Vec<SlotState> = schedule.slots()[3..8].iter().map(|s| s.state).collect();
        assert_eq!(in_flight, vec![SlotState::InFlight; 5]);
        assert_eq!(schedule.in_flight(), 5);
        assert!(
            schedule.slots()[8..]
                .iter()
                .all(|slot| slot.end == Some(SlotEnd::Skipped))
        );
        // The ones being read finish normally afterwards.
        assert!(schedule.finish(3, StreamFetchEnd::Completed).is_empty());
        assert_eq!(schedule.slots()[3].end, Some(SlotEnd::Completed));
        assert!(!schedule.is_done());
    }

    // ---- U3 (u3-give-up-rule): the count and giving up, owned here ------
    // (BR3.1-BR3.6, ADR-007; nfr-design/security-design.md)

    #[test]
    fn three_exhausted_streams_in_a_row_trip_the_limit() {
        // Moved from retry.rs with the streak (BR3.6).
        let mut streak = ExhaustionStreak::new(GIVE_UP_LIMIT);
        streak.record(SlotEnd::FailedExhausted);
        streak.record(SlotEnd::FailedExhausted);
        assert!(!streak.limit_reached());
        streak.record(SlotEnd::FailedExhausted);
        assert!(streak.limit_reached());
        assert_eq!(streak.consecutive(), 3);
    }

    #[test]
    fn a_success_or_a_non_exhausting_failure_resets_the_streak() {
        // Moved from retry.rs with the streak (BR3.6).
        let mut streak = ExhaustionStreak::new(3);
        streak.record(SlotEnd::FailedExhausted);
        streak.record(SlotEnd::FailedExhausted);
        streak.record(SlotEnd::Completed);
        assert_eq!(streak.consecutive(), 0);
        streak.record(SlotEnd::FailedExhausted);
        streak.record(SlotEnd::FailedExhausted);
        streak.record(SlotEnd::FailedNotExhausted);
        assert_eq!(streak.consecutive(), 0);
        streak.record(SlotEnd::FailedExhausted);
        assert!(!streak.limit_reached());
    }

    #[test]
    fn the_give_up_limit_is_three_and_new_uses_it() {
        assert_eq!(GIVE_UP_LIMIT, 3);
        let mut schedule = StreamSchedule::new(names(10));
        start_all(&mut schedule);
        assert!(schedule.finish(0, exhausted("s00")).is_empty());
        assert!(schedule.finish(1, exhausted("s01")).is_empty());
        assert_eq!(schedule.outcome().give_up, None, "two are not enough");
        let skipped = schedule.finish(2, exhausted("s02"));
        assert_eq!(skipped_names(&skipped), ["s08", "s09"]);

        let mut same = StreamSchedule::with_limit(names(10), GIVE_UP_LIMIT);
        start_all(&mut same);
        same.finish(0, exhausted("s00"));
        same.finish(1, exhausted("s01"));
        assert_eq!(same.finish(2, exhausted("s02")), skipped);
        assert_eq!(same.outcome(), schedule.outcome());
    }

    /// AC2.3.1, FR3.1, BR3.1: two exhausted, a success, then one exhausted
    /// counts 1 and does not give up.
    #[test]
    fn a_success_starts_the_count_again() {
        let mut schedule = StreamSchedule::new(names(10));
        start_all(&mut schedule);
        schedule.finish(4, exhausted("s04"));
        schedule.finish(1, exhausted("s01"));
        assert_eq!(schedule.outcome().exhaustion_streak, 2);
        schedule.finish(6, StreamFetchEnd::Completed);
        assert_eq!(schedule.outcome().exhaustion_streak, 0);
        let skipped = schedule.finish(0, exhausted("s00"));
        assert!(skipped.is_empty());
        assert_eq!(schedule.outcome().exhaustion_streak, 1);
        assert_eq!(schedule.outcome().give_up, None);
        assert_eq!(schedule.skipped_count(), 0);
        assert!(schedule.start_next().is_some(), "s08 still starts");
    }

    /// AC2.3.2, FR3.2, BR3.2, NFR3.7, NFR8.9: the example of
    /// functional-spec.md 2 as corrected by security-design.md (R-01): eight
    /// places, one refilled after each of the first five ends; s06, s07 and
    /// s08 end together, so s09-s13 are being read and s14-s20 are not
    /// started when s08 makes three in a row.
    #[test]
    fn the_third_exhausted_stream_skips_the_unstarted_ones_in_the_selected_order() {
        let selected: Vec<String> = (1..=20).map(|i| format!("s{i:02}")).collect();
        let mut schedule = StreamSchedule::new(selected);
        assert_eq!(start_all(&mut schedule), (0..8).collect::<Vec<_>>());
        // Finishing order with the count after each (functional-spec 2).
        let first_five = [
            (1, exhausted("s02"), 1),
            (4, exhausted("s05"), 2),
            (0, StreamFetchEnd::Completed, 0),
            (2, exhausted("s03"), 1),
            (3, denied("s04"), 0),
        ];
        for (order, end, count) in first_five {
            assert!(schedule.finish(order, end).is_empty());
            assert_eq!(schedule.outcome().exhaustion_streak, count);
            assert!(schedule.start_next().is_some());
        }
        assert!(schedule.finish(5, exhausted("s06")).is_empty());
        assert!(schedule.finish(6, exhausted("s07")).is_empty());
        assert_eq!(schedule.outcome().exhaustion_streak, 2);
        let third = failure(FailureKind::Throttled, "s08");
        let skipped = schedule.finish(7, exhausted_with(&third));

        let expected: Vec<String> = (14..=20).map(|i| format!("s{i:02}")).collect();
        assert_eq!(skipped_names(&skipped), expected, "in the selected order");
        assert!(
            skipped.iter().all(|stream| stream.failure == third),
            "the third exhausted stream's failure is copied as it is"
        );
        assert_eq!(
            schedule.outcome().give_up,
            Some(GiveUp {
                triggered_by: "s08".to_string(),
                failure: third.clone(),
                skipped_count: 7,
            })
        );
        assert!(
            schedule.slots()[8..13]
                .iter()
                .all(|slot| slot.state == SlotState::InFlight),
            "s09-s13 are still being read"
        );
        assert!(
            schedule.slots()[13..]
                .iter()
                .all(|slot| slot.state == SlotState::Failed && slot.end == Some(SlotEnd::Skipped))
        );
        assert!(schedule.start_next().is_none(), "nothing more is started");
        assert_eq!(schedule.skipped_count(), 7);
        assert_eq!(schedule.in_flight(), 5);
    }

    /// AC2.3.3, FR3.3, BR3.3: after giving up, the streams being read end
    /// as they end, but the count no longer moves and there is no second
    /// give-up.
    #[test]
    fn after_giving_up_the_streams_being_read_never_move_the_count() {
        let mut schedule = StreamSchedule::new(names(10));
        start_all(&mut schedule);
        schedule.finish(0, exhausted("s00"));
        schedule.finish(1, exhausted("s01"));
        let skipped = schedule.finish(2, exhausted("s02"));
        assert_eq!(skipped_names(&skipped), ["s08", "s09"]);
        let given_up = schedule.outcome().give_up;
        assert_eq!(
            given_up.as_ref().map(|g| g.triggered_by.as_str()),
            Some("s02")
        );

        assert!(schedule.finish(3, StreamFetchEnd::Completed).is_empty());
        assert_eq!(schedule.slots()[3].end, Some(SlotEnd::Completed));
        assert_eq!(schedule.outcome().exhaustion_streak, 3, "not reset");
        assert!(schedule.finish(4, exhausted("s04")).is_empty());
        assert!(schedule.finish(5, exhausted("s05")).is_empty());
        assert!(schedule.finish(6, exhausted("s06")).is_empty());
        assert_eq!(schedule.slots()[6].end, Some(SlotEnd::FailedExhausted));
        assert_eq!(schedule.outcome().exhaustion_streak, 3, "not counted");
        assert!(schedule.finish(7, denied("s07")).is_empty());
        assert_eq!(schedule.outcome().exhaustion_streak, 3);
        assert!(schedule.is_done());
        assert_eq!(schedule.outcome().give_up, given_up, "given up once");
        assert_eq!(schedule.skipped_count(), 2);
        assert_eq!(schedule.finished_count(), 10);
    }

    /// AC2.3.4, FR3.4, BR3.4: a failure that is not retried resets the
    /// count to 0.
    #[test]
    fn a_failure_without_exhaustion_resets_the_count() {
        let mut schedule = StreamSchedule::new(names(10));
        start_all(&mut schedule);
        schedule.finish(0, exhausted("s00"));
        schedule.finish(1, exhausted("s01"));
        schedule.finish(2, denied("s02"));
        assert_eq!(schedule.outcome().exhaustion_streak, 0);
        assert!(schedule.finish(3, exhausted("s03")).is_empty());
        assert!(schedule.finish(4, exhausted("s04")).is_empty());
        assert_eq!(schedule.outcome().exhaustion_streak, 2);
        assert_eq!(schedule.outcome().give_up, None);
        assert_eq!(schedule.skipped_count(), 0);
    }

    /// BR3.5: a stream stopped by the close neither adds to the count nor
    /// resets it.
    #[test]
    fn a_stream_stopped_by_the_close_is_not_counted() {
        let mut schedule = StreamSchedule::new(names(10));
        start_all(&mut schedule);
        schedule.finish(0, exhausted("s00"));
        schedule.finish(1, exhausted("s01"));
        assert!(schedule.finish(2, StreamFetchEnd::Aborted).is_empty());
        assert_eq!(schedule.slots()[2].state, SlotState::Failed);
        assert_eq!(schedule.slots()[2].end, Some(SlotEnd::Aborted));
        assert_eq!(schedule.outcome().exhaustion_streak, 2, "unchanged");
        let skipped = schedule.finish(3, exhausted("s03"));
        assert_eq!(skipped_names(&skipped), ["s08", "s09"], "the third");
    }

    /// Review R-04: giving up with no stream left to skip is still recorded.
    #[test]
    fn giving_up_with_nothing_unstarted_is_still_recorded() {
        let mut schedule = StreamSchedule::new(names(3));
        start_all(&mut schedule);
        schedule.finish(0, exhausted("s00"));
        schedule.finish(1, exhausted("s01"));
        let last = failure(FailureKind::Network, "s02");
        let skipped = schedule.finish(2, exhausted_with(&last));
        assert!(skipped.is_empty());
        assert_eq!(
            schedule.outcome().give_up,
            Some(GiveUp {
                triggered_by: "s02".to_string(),
                failure: last,
                skipped_count: 0,
            })
        );
        assert!(schedule.is_done());
    }

    /// A limit below 1 is taken as 1 (security-design.md, BR3.6).
    #[test]
    fn a_limit_below_one_counts_as_one() {
        let mut schedule = StreamSchedule::with_limit(names(10), 0);
        start_all(&mut schedule);
        let skipped = schedule.finish(5, exhausted("s05"));
        assert_eq!(skipped_names(&skipped), ["s08", "s09"]);
        assert_eq!(
            schedule
                .outcome()
                .give_up
                .map(|g| (g.triggered_by, g.skipped_count)),
            Some(("s05".to_string(), 2))
        );

        let mut two = StreamSchedule::with_limit(names(10), 2);
        start_all(&mut two);
        assert!(two.finish(0, exhausted("s00")).is_empty());
        assert_eq!(two.finish(1, exhausted("s01")).len(), 2);
    }
}
