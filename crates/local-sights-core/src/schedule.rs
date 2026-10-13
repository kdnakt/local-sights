//! StreamSchedule: which stream to read next, at most 8 at a time
//! (261012-parallel-fetch U1:BR1.7, BR1.8, BR1.13, BR1.19).
//!
//! The schedule is pure bookkeeping: it never calls AWS or waits. The
//! FetchCoordinator asks it for the next stream whenever a place is free
//! ([`StreamSchedule::start_next`]) and tells it how each stream ended
//! ([`StreamSchedule::finish`]), in the order the streams end.
//!
//! Giving up is the U3 rule applied in finishing order, for now (U1:BR1.13,
//! ADR-007; U3 of this intent refines it): the existing [`ExhaustionStreak`]
//! counts streams that used up their retries in a row; once it reaches the
//! limit, every stream not started yet is failed without calling the API
//! (Skipped), with the failure of the last exhausted stream. Streams being
//! read are read to their end.

use crate::failure::ApiFailure;
use crate::retry::{ExhaustionStreak, StreamEnding};

/// Streams read at the same time (U1:BR1.7, NFR5.3). Fixed: neither the
/// settings nor the screen change it.
pub const MAX_IN_FLIGHT: usize = 8;

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
    /// used up their retries (U1:BR1.13).
    Skipped,
}

impl SlotEnd {
    /// The ending as the exhaustion streak counts it; `None` when stopped
    /// or skipped.
    fn streak_ending(self) -> Option<StreamEnding> {
        match self {
            SlotEnd::Completed => Some(StreamEnding::Succeeded),
            SlotEnd::FailedExhausted => Some(StreamEnding::Exhausted),
            SlotEnd::FailedNotExhausted => Some(StreamEnding::FailedWithoutExhaustion),
            SlotEnd::Aborted | SlotEnd::Skipped => None,
        }
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

/// A stream failed without calling the API (U1:BR1.13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedStream {
    /// Stream name.
    pub log_stream_name: String,
    /// The failure of the last stream that used up its retries.
    pub failure: ApiFailure,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleOutcome {
    /// Completed or CompletedWithFailures.
    pub end_state: ScheduleEndState,
    /// Streams that used up their retries in a row, in finishing order.
    pub exhaustion_streak: u32,
    /// Streams being read or not started yet.
    pub unfinished_streams: usize,
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
    last_exhausted_failure: Option<ApiFailure>,
}

impl StreamSchedule {
    /// A schedule of `names`, in the selected order (U1:BR1.8); the streams
    /// left are skipped after `max_consecutive_exhausted` exhausted streams
    /// in a row (U1:BR1.13).
    pub fn new(names: Vec<String>, max_consecutive_exhausted: u32) -> Self {
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
            streak: ExhaustionStreak::new(max_consecutive_exhausted),
            last_exhausted_failure: None,
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

    /// Records how the stream at `order` ended, in finishing order
    /// (U1:BR1.13, BR1.19). `failure` is the stream's failure; it is kept
    /// when the stream used up its retries. When this makes too many
    /// exhausted streams in a row, every unstarted stream becomes Skipped
    /// and is returned, in order, with that failure. A stream that is not
    /// being read is left as it is.
    pub fn finish(
        &mut self,
        order: usize,
        end: SlotEnd,
        failure: Option<&ApiFailure>,
    ) -> Vec<SkippedStream> {
        let Some(slot) = self.slots.get_mut(order) else {
            return Vec::new();
        };
        if slot.state != SlotState::InFlight || end == SlotEnd::Skipped {
            return Vec::new();
        }
        slot.state = if end == SlotEnd::Completed {
            SlotState::Succeeded
        } else {
            SlotState::Failed
        };
        slot.end = Some(end);
        self.in_flight -= 1;
        self.finished += 1;
        if end != SlotEnd::Completed {
            self.failed += 1;
        }
        if let Some(ending) = end.streak_ending() {
            self.streak.record(ending);
        }
        if end == SlotEnd::FailedExhausted {
            self.last_exhausted_failure = failure.cloned();
        }
        self.skip_unstarted_when_limit_reached()
    }

    /// U1:BR1.13: fails every unstarted stream without calling.
    fn skip_unstarted_when_limit_reached(&mut self) -> Vec<SkippedStream> {
        let (true, Some(failure)) = (
            self.streak.limit_reached(),
            self.last_exhausted_failure.clone(),
        ) else {
            return Vec::new();
        };
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure::{ApiFailure, FailureKind, SafeDetailFields};

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

    fn start_all(schedule: &mut StreamSchedule) -> Vec<usize> {
        std::iter::from_fn(|| schedule.start_next().map(|(order, _)| order)).collect()
    }

    #[test]
    fn next_hands_out_the_streams_in_the_selected_order() {
        let mut schedule = StreamSchedule::new(names(3), 3);
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
        let mut schedule = StreamSchedule::new(names(10), 3);
        assert_eq!(start_all(&mut schedule), (0..8).collect::<Vec<_>>());
        assert_eq!(MAX_IN_FLIGHT, 8);
        assert_eq!(schedule.in_flight(), 8);
        assert!(schedule.start_next().is_none(), "the ninth waits");
        assert_eq!(schedule.slots()[8].state, SlotState::Unstarted);
    }

    #[test]
    fn a_finished_stream_frees_a_place_for_the_next_one() {
        let mut schedule = StreamSchedule::new(names(10), 3);
        start_all(&mut schedule);
        let skipped = schedule.finish(5, SlotEnd::Completed, None);
        assert!(skipped.is_empty());
        assert_eq!(schedule.slots()[5].state, SlotState::Succeeded);
        assert_eq!(schedule.slots()[5].end, Some(SlotEnd::Completed));
        assert_eq!(schedule.start_next().map(|(order, _)| order), Some(8));
        assert!(schedule.start_next().is_none());
        schedule.finish(0, SlotEnd::FailedNotExhausted, None);
        assert_eq!(schedule.slots()[0].state, SlotState::Failed);
        assert_eq!(schedule.start_next().map(|(order, _)| order), Some(9));
        assert!(!schedule.is_done());
    }

    #[test]
    fn no_stream_means_nothing_to_read_and_done_at_once() {
        let mut schedule = StreamSchedule::new(Vec::new(), 3);
        assert!(schedule.start_next().is_none());
        assert!(schedule.is_done());
        assert_eq!(
            schedule.outcome(),
            ScheduleOutcome {
                end_state: ScheduleEndState::Completed,
                exhaustion_streak: 0,
                unfinished_streams: 0,
            }
        );
    }

    #[test]
    fn three_exhausted_in_finishing_order_skip_every_unstarted_stream() {
        let mut schedule = StreamSchedule::new(names(11), 3);
        start_all(&mut schedule);
        // Finishing order 6, 2, 7 (not the reading order).
        assert!(
            schedule
                .finish(
                    6,
                    SlotEnd::FailedExhausted,
                    Some(&failure(FailureKind::Throttled, "s06"))
                )
                .is_empty()
        );
        assert_eq!(schedule.start_next().map(|(order, _)| order), Some(8));
        assert!(
            schedule
                .finish(
                    2,
                    SlotEnd::FailedExhausted,
                    Some(&failure(FailureKind::Network, "s02"))
                )
                .is_empty()
        );
        assert_eq!(schedule.start_next().map(|(order, _)| order), Some(9));
        let last = failure(FailureKind::Throttled, "s07");
        let skipped = schedule.finish(7, SlotEnd::FailedExhausted, Some(&last));
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
        let mut schedule = StreamSchedule::new(names(9), 3);
        start_all(&mut schedule);
        let exhausted = failure(FailureKind::Throttled, "x");
        schedule.finish(0, SlotEnd::FailedExhausted, Some(&exhausted));
        schedule.finish(1, SlotEnd::FailedExhausted, Some(&exhausted));
        schedule.finish(2, SlotEnd::Completed, None);
        assert_eq!(schedule.outcome().exhaustion_streak, 0);
        schedule.finish(3, SlotEnd::FailedExhausted, Some(&exhausted));
        schedule.finish(4, SlotEnd::FailedExhausted, Some(&exhausted));
        schedule.finish(5, SlotEnd::FailedNotExhausted, None);
        assert_eq!(schedule.outcome().exhaustion_streak, 0);
        let skipped = schedule.finish(6, SlotEnd::FailedExhausted, Some(&exhausted));
        assert!(skipped.is_empty());
        assert_eq!(schedule.outcome().exhaustion_streak, 1);
        assert!(schedule.start_next().is_some(), "the ninth still starts");
    }

    #[test]
    fn finished_and_skipped_streams_are_counted() {
        let mut schedule = StreamSchedule::new(names(12), 3);
        start_all(&mut schedule);
        assert_eq!(
            (schedule.finished_count(), schedule.skipped_count()),
            (0, 0)
        );
        let exhausted = failure(FailureKind::Throttled, "x");
        schedule.finish(0, SlotEnd::FailedExhausted, Some(&exhausted));
        schedule.finish(1, SlotEnd::FailedExhausted, Some(&exhausted));
        // Two places freed but not refilled yet: s08 and s09 stay unstarted.
        let skipped = schedule.finish(2, SlotEnd::FailedExhausted, Some(&exhausted));
        assert_eq!(skipped.len(), 4, "s08 to s11");
        assert_eq!(schedule.skipped_count(), 4);
        assert_eq!(schedule.finished_count(), 7, "three read and four skipped");
        assert_eq!(
            schedule.outcome(),
            ScheduleOutcome {
                end_state: ScheduleEndState::CompletedWithFailures,
                exhaustion_streak: 3,
                unfinished_streams: 5,
            }
        );
        for order in 3..8 {
            schedule.finish(order, SlotEnd::Completed, None);
        }
        assert_eq!(schedule.finished_count(), 12);
        assert!(schedule.is_done());
        assert_eq!(schedule.outcome().unfinished_streams, 0);
    }

    #[test]
    fn streams_being_read_are_never_skipped() {
        let mut schedule = StreamSchedule::new(names(10), 3);
        start_all(&mut schedule);
        let exhausted = failure(FailureKind::Throttled, "x");
        for order in 0..3 {
            schedule.finish(order, SlotEnd::FailedExhausted, Some(&exhausted));
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
        assert!(schedule.finish(3, SlotEnd::Completed, None).is_empty());
        assert_eq!(schedule.slots()[3].end, Some(SlotEnd::Completed));
        assert!(!schedule.is_done());
    }
}
