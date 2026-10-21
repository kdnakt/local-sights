//! QueryStore: the query screen's own holdings, per log group
//! (261017-insights-query U3:BR1.1-BR1.6, BR3.2, BR3.3, entities.md
//! QueryStore, GroupHoldings, ViewEntry).
//!
//! The holdings are not the Log Management screen's EventTimeline and
//! nothing is passed between the two (FR2.1, RA Q3b, U1:BR1.7). For every
//! (profile, region, log group) they keep:
//!
//! - the committed events, taken as fetched, with the ranges they cover
//!   (no two ranges share a millisecond, adjacent ones are joined);
//! - the staging events: fetched by this run but not taken as fetched (a
//!   log group where a stream failed or [Cancel] stopped the fetch, and the
//!   newest five minutes of a range, BR1.2); they serve this run's result
//!   and are discarded when the next run starts (BR1.3), so the next run
//!   fetches their part again and no event is held twice (FR2.7);
//! - the next order number of every stream, so the events of two fetches
//!   never share a number (BR1.4, [`StreamSequences`]).
//!
//! Since U1 a run went through `run_fetch_with_cache` straight into this
//! store; since U3 every range of a run is fetched into a holder of its own
//! (an `EventTimeline` behind a lock) and only the finished range is handed
//! here and split into committed and staging (functional-design review
//! R-01), so the fetch's own discard empties that holder only.
//!
//! Every run builds its view, the events of the chosen log groups that lie
//! in its period, committed and staging, in time order (BR1.5); the
//! evaluation reads it step by step, each step under one lock. When the
//! evaluation ends the result rows are copied out with their log group, so
//! a result can be read whatever the holdings become afterwards (BR3.2).
//! Every run start and every connection change increments the epoch; the
//! view is only read while the epoch is the one it was built for.
//!
//! A log group no longer chosen loses its holdings when the next run starts
//! (BR1.3, 261020-carryover-fixes FR2.2.1); every holding goes when the
//! connection changes or the app closes (BR1.6).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::cache::plan::{
    CacheKey, CachedEvent, CoveredRange, merge_ranges, missing_ranges, record_range,
};
use crate::catalog::ProfileSelector;
use crate::event::LogEvent;
use crate::query::evaluator::{EvalItem, EvalItems, Evaluation, ResultRow};
use crate::query::fields::{FIELD_NAME_SCAN_EVENTS, collect_json_field_names};
use crate::query::plan::QueryPlan;
use crate::timeline::EventTimeline;

/// Which holding of a log group an event goes to (entities.md
/// GroupHoldings, 261017-insights-query U3:BR1.2, BR1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holding {
    /// Taken as fetched: kept until the connection changes or a run starts
    /// without its log group (261020-carryover-fixes FR2.2.1).
    Committed,
    /// Used for this run only, discarded when the next run starts.
    Staging,
}

/// The next order number of every stream of one log group (entities.md
/// GroupHoldings.nextSequence, 261017-insights-query U3:BR1.4, review R-05).
///
/// Every fetch and every cache read numbers its events from 0 per stream,
/// so a log group's holdings number them again as they are added: using a
/// number twice would leave a stream and a number no longer pointing at one
/// event. Committed and staging events share one series per stream; when
/// the staging events go, the series goes back to the number after the last
/// committed one, since no event uses the numbers after it any more, so
/// fetching the same part again and again never makes the numbers grow.
#[derive(Debug, Clone, Default)]
pub struct StreamSequences {
    next: HashMap<String, u64>,
    after_committed: HashMap<String, u64>,
}

impl StreamSequences {
    /// No stream numbered yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Numbers `events` (in time order, as fetched or read) per stream
    /// from where each stream's series stands, in their order, for
    /// `holding`.
    pub fn number(&mut self, mut events: Vec<LogEvent>, holding: Holding) -> Vec<LogEvent> {
        for event in &mut events {
            event.sequence = self.take(&event.log_stream_name, holding);
        }
        events
    }

    /// Numbers cached events, which are committed (U3:BR2.1).
    pub fn number_cached(&mut self, events: Vec<CachedEvent>) -> Vec<LogEvent> {
        events
            .into_iter()
            .map(|event| LogEvent {
                sequence: self.take(&event.log_stream_name, Holding::Committed),
                timestamp: event.timestamp,
                ingestion_time: event.ingestion_time,
                message: event.message,
                log_stream_name: event.log_stream_name,
            })
            .collect()
    }

    /// The staging events were discarded: every series goes back to the
    /// number after its last committed one.
    pub fn discard_staging(&mut self) {
        self.next.clone_from(&self.after_committed);
    }

    /// The number the next event of `stream` gets.
    pub fn next_of(&self, stream: &str) -> u64 {
        self.next.get(stream).copied().unwrap_or(0)
    }

    /// The next number of `stream`, moving its series on.
    fn take(&mut self, stream: &str, holding: Holding) -> u64 {
        let number = self.next_of(stream);
        // Look the stream up first, so its name is copied only when new.
        match self.next.get_mut(stream) {
            Some(next) => *next = number + 1,
            None => {
                self.next.insert(stream.to_string(), number + 1);
            }
        }
        if holding == Holding::Committed {
            match self.after_committed.get_mut(stream) {
                Some(after) => *after = number + 1,
                None => {
                    self.after_committed.insert(stream.to_string(), number + 1);
                }
            }
        }
        number
    }
}

/// What a log group's holdings are for (entities.md HoldingKey).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HoldingKey {
    /// The profile of the connection.
    pub profile: ProfileSelector,
    /// The region of the connection.
    pub region: String,
    /// The log group, the value of `@log` (U1:BR4.1).
    pub log_group_name: String,
}

impl HoldingKey {
    /// The disk cache's key of the same log group (U3:BR2.1).
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::new(
            self.profile.clone(),
            self.region.clone(),
            self.log_group_name.clone(),
        )
    }
}

/// One log group's holdings (entities.md GroupHoldings).
#[derive(Debug, Default)]
struct GroupHoldings {
    committed: EventTimeline,
    covered: Vec<CoveredRange>,
    staging: EventTimeline,
    sequences: StreamSequences,
}

impl GroupHoldings {
    /// U3:BR1.3: the staging events go, and their numbers with them.
    fn discard_staging(&mut self) {
        if !self.staging.is_empty() {
            self.staging.clear();
        }
        self.sequences.discard_staging();
    }

    /// Numbers `events` and adds them to `holding` (U3:BR1.4).
    fn add(&mut self, events: Vec<LogEvent>, holding: Holding) {
        if events.is_empty() {
            return;
        }
        let numbered = self.sequences.number(events, holding);
        match holding {
            Holding::Committed => self.committed.append(numbered),
            Holding::Staging => self.staging.append(numbered),
        };
    }

    fn holding(&self, source: Holding) -> &[LogEvent] {
        match source {
            Holding::Committed => self.committed.events(),
            Holding::Staging => self.staging.events(),
        }
    }

    fn len(&self) -> usize {
        self.committed.len() + self.staging.len()
    }
}

/// One event of a run's view (entities.md ViewEntry): the chosen log group
/// it belongs to (by its place in the run's choice), its holding and its
/// place there. Valid while the epoch stays the view's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewEntry {
    /// The log group's place in the run's choice.
    pub group: usize,
    /// Committed or staging.
    pub source: Holding,
    /// The event's place in that holding.
    pub position: usize,
}

/// The holdings behind the lock.
#[derive(Debug, Default)]
struct Holdings {
    groups: HashMap<HoldingKey, GroupHoldings>,
    epoch: u64,
    /// The log groups of the current run, in the order they were chosen.
    run_keys: Vec<HoldingKey>,
    /// The current run's view (U3:BR1.5), built for `view_epoch`.
    view: Vec<ViewEntry>,
    view_epoch: Option<u64>,
}

/// The events of a run's view, lent to the evaluator by position.
struct RunItems<'a> {
    view: &'a [ViewEntry],
    groups: Vec<(&'a GroupHoldings, &'a str)>,
}

impl<'a> RunItems<'a> {
    fn of(holdings: &'a Holdings) -> Self {
        Self {
            view: &holdings.view,
            groups: holdings
                .run_keys
                .iter()
                .filter_map(|key| {
                    holdings
                        .groups
                        .get(key)
                        .map(|group| (group, key.log_group_name.as_str()))
                })
                .collect(),
        }
    }

    fn entry(&self, entry: ViewEntry) -> Option<EvalItem<'a>> {
        let (group, log_group_name) = self.groups.get(entry.group).copied()?;
        let event = group.holding(entry.source).get(entry.position)?;
        Some(EvalItem {
            event,
            log_group_name,
        })
    }
}

impl EvalItems for RunItems<'_> {
    fn len(&self) -> usize {
        self.view.len()
    }

    fn item(&self, position: usize) -> Option<EvalItem<'_>> {
        self.entry(*self.view.get(position)?)
    }
}

/// The query screen's holdings, behind one lock.
#[derive(Debug, Default)]
pub struct QueryStore {
    holdings: Mutex<Holdings>,
}

impl QueryStore {
    /// No holdings, epoch 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// A poisoned lock still holds consistent holdings (every change is one
    /// step), so it is recovered.
    fn lock(&self) -> MutexGuard<'_, Holdings> {
        self.holdings.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// U3:BR1.3: a run of `keys` (in the order chosen) starts. The holdings
    /// of every log group not in `keys` go, events and ranges alike, so
    /// choosing it again fetches it again (261020-carryover-fixes FR2.2.1,
    /// FR2.2.2); the staging events of the chosen ones go, their committed
    /// events and ranges stay (FR2.2.3). The epoch goes up and the previous
    /// view with it. Returns the new epoch.
    pub fn begin_run(&self, keys: Vec<HoldingKey>) -> u64 {
        let mut holdings = self.lock();
        holdings.groups.retain(|key, _| keys.contains(key));
        for group in holdings.groups.values_mut() {
            group.discard_staging();
        }
        for key in &keys {
            holdings.groups.entry(key.clone()).or_default();
        }
        holdings.run_keys = keys;
        holdings.view = Vec::new();
        holdings.view_epoch = None;
        holdings.epoch += 1;
        holdings.epoch
    }

    /// U3:BR1.6: the connection changed; every log group's holdings and
    /// ranges go. Returns the new epoch.
    pub fn discard_all(&self) -> u64 {
        let mut holdings = self.lock();
        holdings.groups.clear();
        holdings.run_keys.clear();
        holdings.view = Vec::new();
        holdings.view_epoch = None;
        holdings.epoch += 1;
        holdings.epoch
    }

    /// The current epoch.
    pub fn epoch(&self) -> u64 {
        self.lock().epoch
    }

    /// Number of held events of every log group, committed and staging.
    pub fn len(&self) -> usize {
        self.lock().groups.values().map(GroupHoldings::len).sum()
    }

    /// Whether no event is held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The committed ranges of `key`, earliest first.
    pub fn covered_ranges(&self, key: &HoldingKey) -> Vec<CoveredRange> {
        self.lock()
            .groups
            .get(key)
            .map(|group| group.covered.clone())
            .unwrap_or_default()
    }

    /// The held events of `key` as `(committed, staging)` counts.
    pub fn counts(&self, key: &HoldingKey) -> (usize, usize) {
        self.lock()
            .groups
            .get(key)
            .map_or((0, 0), |group| (group.committed.len(), group.staging.len()))
    }

    /// U3:BR1.1: the parts of `[start_ms, end_ms]` that `key`'s committed
    /// ranges do not hold.
    pub fn missing_ranges(
        &self,
        key: &HoldingKey,
        start_ms: i64,
        end_ms: i64,
    ) -> Vec<CoveredRange> {
        let holdings = self.lock();
        let covered = holdings
            .groups
            .get(key)
            .map_or(&[][..], |group| group.covered.as_slice());
        missing_ranges(start_ms, end_ms, covered)
    }

    /// U3:BR2.1: the events the disk cache holds for `ranges` of `key` are
    /// committed and the ranges covered. The cache records no part newer
    /// than five minutes before the fetch that wrote it, so BR1.2 holds.
    /// Only events in `ranges` are taken.
    pub fn add_cached(&self, key: &HoldingKey, ranges: &[CoveredRange], events: Vec<CachedEvent>) {
        let mut holdings = self.lock();
        let group = holdings.groups.entry(key.clone()).or_default();
        let inside: Vec<CachedEvent> = events
            .into_iter()
            .filter(|event| {
                ranges
                    .iter()
                    .any(|range| range.contains_instant(event.timestamp))
            })
            .collect();
        let numbered = group.sequences.number_cached(inside);
        group.committed.append(numbered);
        for range in ranges {
            group.covered = merge_ranges(&group.covered, *range);
        }
    }

    /// U3:BR1.2, review R-01: the fetch of `range` of `key` ended and handed
    /// its `events` over. When it `completed` (every stream read), the part
    /// up to five minutes before `run_started_at_ms` is committed and
    /// covered and the rest is staging (R-07: one boundary for every log
    /// group of the run); otherwise everything is staging. An event outside
    /// `range` is dropped. Returns the range now covered, if any.
    pub fn settle_range(
        &self,
        key: &HoldingKey,
        range: CoveredRange,
        events: Vec<LogEvent>,
        completed: bool,
        run_started_at_ms: i64,
    ) -> Option<CoveredRange> {
        let committed = if completed {
            record_range(range.start_ms, range.end_ms, run_started_at_ms)
        } else {
            None
        };
        let (taken, staged): (Vec<LogEvent>, Vec<LogEvent>) = events
            .into_iter()
            .filter(|event| range.contains_instant(event.timestamp))
            .partition(|event| {
                committed.is_some_and(|part| part.contains_instant(event.timestamp))
            });
        let mut holdings = self.lock();
        let group = holdings.groups.entry(key.clone()).or_default();
        group.add(taken, Holding::Committed);
        if let Some(part) = committed {
            group.covered = merge_ranges(&group.covered, part);
        }
        group.add(staged, Holding::Staging);
        committed
    }

    /// U3:BR1.5: builds the view of the run of `epoch`: of every chosen log
    /// group, the committed and staging events in `[start_ms, end_ms]`, in
    /// `(timestamp, log group, stream, order)` order. Returns its length, or
    /// `None` when the epoch is no longer the run's.
    pub fn build_view(&self, epoch: u64, start_ms: i64, end_ms: i64) -> Option<usize> {
        let mut holdings = self.lock();
        if holdings.epoch != epoch {
            return None;
        }
        let mut view = Vec::new();
        for (group_index, key) in holdings.run_keys.iter().enumerate() {
            let Some(group) = holdings.groups.get(key) else {
                continue;
            };
            for source in [Holding::Committed, Holding::Staging] {
                let events = group.holding(source);
                let first = events.partition_point(|event| event.timestamp < start_ms);
                let last = events.partition_point(|event| event.timestamp <= end_ms);
                view.extend((first..last.max(first)).map(|position| ViewEntry {
                    group: group_index,
                    source,
                    position,
                }));
            }
        }
        // Each holding is in time order already: the sort merges the runs.
        let items = RunItems {
            view: &[],
            groups: holdings
                .run_keys
                .iter()
                .filter_map(|key| {
                    holdings
                        .groups
                        .get(key)
                        .map(|group| (group, key.log_group_name.as_str()))
                })
                .collect(),
        };
        view.sort_by(
            |left, right| match (items.entry(*left), items.entry(*right)) {
                (Some(left), Some(right)) => (
                    left.event.timestamp,
                    left.log_group_name,
                    left.event.log_stream_name.as_str(),
                    left.event.sequence,
                )
                    .cmp(&(
                        right.event.timestamp,
                        right.log_group_name,
                        right.event.log_stream_name.as_str(),
                        right.event.sequence,
                    )),
                _ => std::cmp::Ordering::Equal,
            },
        );
        drop(items);
        let length = view.len();
        holdings.view = view;
        holdings.view_epoch = Some(epoch);
        Some(length)
    }

    /// Number of events in the view of `epoch`, or `None` for another
    /// epoch.
    pub fn view_len(&self, epoch: u64) -> Option<usize> {
        let holdings = self.lock();
        (holdings.view_epoch == Some(epoch)).then_some(holdings.view.len())
    }

    /// U1:BR5.8, U2:BR3.1: does the next step of `evaluation` over the view
    /// under one lock. Returns `false`, doing nothing, when the view is no
    /// longer that of `epoch` (a newer run or a connection change).
    pub fn evaluate_step(&self, plan: &QueryPlan, evaluation: &mut Evaluation, epoch: u64) -> bool {
        let holdings = self.lock();
        if holdings.view_epoch != Some(epoch) || holdings.epoch != epoch {
            return false;
        }
        evaluation.step(plan, &RunItems::of(&holdings));
        true
    }

    /// U3:BR3.3 (U2:BR1.7): the JSON field names of the first
    /// [`FIELD_NAME_SCAN_EVENTS`] events of the view of `epoch`.
    pub fn json_field_names(&self, epoch: u64) -> Option<Vec<String>> {
        let holdings = self.lock();
        if holdings.view_epoch != Some(epoch) || holdings.epoch != epoch {
            return None;
        }
        let items = RunItems::of(&holdings);
        let messages = (0..items.len().min(FIELD_NAME_SCAN_EVENTS))
            .filter_map(|position| items.item(position))
            .map(|item| item.event.message.as_str());
        Some(collect_json_field_names(messages))
    }

    /// U3:BR3.2: copies the events at `positions` of the view of `epoch`
    /// with their log group, in the order asked, for a result that outlives
    /// the holdings. `None` when the view is no longer that of `epoch`.
    pub fn copy_rows(&self, epoch: u64, positions: &[usize]) -> Option<Vec<ResultRow>> {
        let holdings = self.lock();
        if holdings.view_epoch != Some(epoch) || holdings.epoch != epoch {
            return None;
        }
        let items = RunItems::of(&holdings);
        Some(
            positions
                .iter()
                .filter_map(|position| items.item(*position))
                .map(|item| ResultRow {
                    event: item.event.clone(),
                    log_group_name: item.log_group_name.to_string(),
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_view::LogView;
    use crate::query::parser::parse_query;

    fn key(group: &str) -> HoldingKey {
        HoldingKey {
            profile: ProfileSelector::Named("dev".to_string()),
            region: "ap-northeast-1".to_string(),
            log_group_name: group.to_string(),
        }
    }

    fn event(timestamp: i64, stream: &str, sequence: u64) -> LogEvent {
        LogEvent {
            timestamp,
            ingestion_time: None,
            message: format!("{stream}:{sequence}"),
            log_stream_name: stream.to_string(),
            sequence,
        }
    }

    fn range(start_ms: i64, end_ms: i64) -> CoveredRange {
        CoveredRange { start_ms, end_ms }
    }

    /// The `(timestamp, log group, stream)` of every row of the view of
    /// `epoch`, in the view's order.
    fn view(store: &QueryStore, epoch: u64) -> Vec<(i64, String, String)> {
        let length = store.view_len(epoch).unwrap_or(0);
        let positions: Vec<usize> = (0..length).collect();
        store
            .copy_rows(epoch, &positions)
            .unwrap_or_default()
            .into_iter()
            .map(|row| {
                (
                    row.event.timestamp,
                    row.log_group_name,
                    row.event.log_stream_name,
                )
            })
            .collect()
    }

    fn row(timestamp: i64, group: &str, stream: &str) -> (i64, String, String) {
        (timestamp, group.to_string(), stream.to_string())
    }

    /// Long after every event of the tests: nothing is too new.
    const LATER: i64 = 1_000_000_000;

    // ---- 261017-insights-query U3:BR1.4, review R-05 (test first) ------

    /// The `(stream, sequence)` of `events`, in order.
    fn numbers(events: &[LogEvent]) -> Vec<(String, u64)> {
        events
            .iter()
            .map(|event| (event.log_stream_name.clone(), event.sequence))
            .collect()
    }

    fn numbered(pairs: &[(&str, u64)]) -> Vec<(String, u64)> {
        pairs
            .iter()
            .map(|(stream, sequence)| ((*stream).to_string(), *sequence))
            .collect()
    }

    #[test]
    fn two_fetched_ranges_never_share_a_number_in_a_stream() {
        let mut sequences = StreamSequences::new();
        // Each fetch numbers its events from 0 per stream.
        let first = sequences.number(
            vec![event(10, "a", 0), event(11, "b", 0), event(12, "a", 1)],
            Holding::Committed,
        );
        assert_eq!(numbers(&first), numbered(&[("a", 0), ("b", 0), ("a", 1)]));
        let second = sequences.number(
            vec![event(20, "a", 0), event(21, "a", 1), event(22, "b", 0)],
            Holding::Committed,
        );
        assert_eq!(
            numbers(&second),
            numbered(&[("a", 2), ("a", 3), ("b", 1)]),
            "the series goes on"
        );
        assert_eq!(sequences.next_of("a"), 4);
        assert_eq!(sequences.next_of("c"), 0);
        assert_eq!(second[0].timestamp, 20, "nothing else changes");
    }

    #[test]
    fn the_same_stream_name_in_two_log_groups_is_two_series() {
        // FR2.8: every log group holds its own numbers.
        let mut orders = StreamSequences::new();
        let mut payments = StreamSequences::new();
        orders.number(
            vec![event(1, "s1", 0), event(2, "s1", 1)],
            Holding::Committed,
        );
        let other = payments.number(vec![event(1, "s1", 0)], Holding::Committed);
        assert_eq!(numbers(&other), numbered(&[("s1", 0)]));
        assert_eq!((orders.next_of("s1"), payments.next_of("s1")), (2, 1));
    }

    #[test]
    fn staging_numbers_are_given_again_after_the_staging_events_go() {
        // R-05: fetching the same new part again and again keeps the
        // numbers where they were.
        let mut sequences = StreamSequences::new();
        sequences.number(vec![event(1, "a", 0), event(2, "a", 1)], Holding::Committed);
        for _ in 0..3 {
            let fresh =
                sequences.number(vec![event(3, "a", 0), event(4, "a", 1)], Holding::Staging);
            assert_eq!(numbers(&fresh), numbered(&[("a", 2), ("a", 3)]));
            sequences.discard_staging();
            assert_eq!(sequences.next_of("a"), 2);
        }
        let only_staging = sequences.number(vec![event(9, "z", 0)], Holding::Staging);
        assert_eq!(numbers(&only_staging), numbered(&[("z", 0)]));
        sequences.discard_staging();
        assert_eq!(
            sequences.next_of("z"),
            0,
            "a stream with no committed event"
        );
    }

    #[test]
    fn a_committed_event_after_staging_ones_keeps_its_number_after_the_discard() {
        // A failed earlier range went to staging, a later one was taken:
        // the gap stays, no number is used twice.
        let mut sequences = StreamSequences::new();
        sequences.number(vec![event(1, "a", 0)], Holding::Staging);
        let taken = sequences.number(vec![event(5, "a", 0)], Holding::Committed);
        assert_eq!(numbers(&taken), numbered(&[("a", 1)]));
        sequences.discard_staging();
        let again = sequences.number(vec![event(1, "a", 0)], Holding::Staging);
        assert_eq!(numbers(&again), numbered(&[("a", 2)]));
    }

    #[test]
    fn events_read_from_the_cache_are_numbered_too() {
        let mut sequences = StreamSequences::new();
        sequences.number(vec![event(1, "a", 0)], Holding::Committed);
        let cached = |timestamp, stream: &str, sequence| CachedEvent {
            timestamp,
            ingestion_time: Some(timestamp + 1),
            message: format!("cached {timestamp}"),
            log_stream_name: stream.to_string(),
            sequence,
        };
        let read = sequences.number_cached(vec![
            cached(5, "a", 7),
            cached(6, "b", 3),
            cached(7, "a", 9),
        ]);
        assert_eq!(numbers(&read), numbered(&[("a", 1), ("b", 0), ("a", 2)]));
        assert_eq!(read[0].message, "cached 5");
        assert_eq!(read[0].ingestion_time, Some(6));
        sequences.discard_staging();
        assert_eq!(sequences.next_of("a"), 3, "cached events are committed");
    }

    // ---- 261017-insights-query U3:BR1.1-BR1.6, R-01, R-07 --------------

    #[test]
    fn a_completed_range_is_committed_and_covered() {
        // BR1.2, R-01.
        let store = QueryStore::new();
        let group = key("/g");
        store.begin_run(vec![group.clone()]);
        assert_eq!(
            store.missing_ranges(&group, 100, 199),
            vec![range(100, 199)]
        );
        let covered = store.settle_range(
            &group,
            range(100, 199),
            vec![event(100, "a", 0), event(150, "a", 1)],
            true,
            LATER,
        );
        assert_eq!(covered, Some(range(100, 199)));
        assert_eq!(store.counts(&group), (2, 0));
        assert_eq!(store.covered_ranges(&group), vec![range(100, 199)]);
        assert_eq!(
            store.missing_ranges(&group, 50, 250),
            vec![range(50, 99), range(200, 250)]
        );
    }

    #[test]
    fn the_newest_five_minutes_of_a_completed_range_are_staging() {
        // BR1.2, R-07: the boundary is five minutes before the run started.
        let store = QueryStore::new();
        let group = key("/g");
        let started = 1_000_000;
        let boundary = started - 300_000;
        store.begin_run(vec![group.clone()]);
        let covered = store.settle_range(
            &group,
            range(0, started),
            vec![
                event(10, "a", 0),
                event(boundary, "a", 1),
                event(boundary + 1, "a", 2),
            ],
            true,
            started,
        );
        assert_eq!(covered, Some(range(0, boundary)));
        assert_eq!(
            store.counts(&group),
            (2, 1),
            "the boundary millisecond is committed"
        );
        assert_eq!(
            store.missing_ranges(&group, 0, started),
            vec![range(boundary + 1, started)]
        );
    }

    #[test]
    fn a_range_not_completed_is_all_staging_and_covers_nothing() {
        // BR1.2: CompletedWithFailures, Failed and Cancelled.
        let store = QueryStore::new();
        let group = key("/g");
        store.begin_run(vec![group.clone()]);
        let covered = store.settle_range(
            &group,
            range(100, 199),
            vec![event(120, "a", 0), event(300, "a", 1)],
            false,
            LATER,
        );
        assert_eq!(covered, None);
        assert_eq!(
            store.counts(&group),
            (0, 1),
            "the event outside the range is dropped"
        );
        assert_eq!(
            store.missing_ranges(&group, 100, 199),
            vec![range(100, 199)]
        );
    }

    #[test]
    fn the_next_run_discards_the_staging_events_and_fetches_their_part_again() {
        // BR1.3, R-05: the numbers stay where they were.
        let store = QueryStore::new();
        let group = key("/g");
        store.begin_run(vec![group.clone()]);
        store.settle_range(
            &group,
            range(100, 199),
            vec![event(100, "a", 0)],
            true,
            LATER,
        );
        store.settle_range(
            &group,
            range(200, 299),
            vec![event(250, "a", 0)],
            false,
            LATER,
        );
        assert_eq!(store.counts(&group), (1, 1));
        let epoch = store.begin_run(vec![group.clone()]);
        assert_eq!(store.counts(&group), (1, 0));
        assert_eq!(
            store.missing_ranges(&group, 100, 299),
            vec![range(200, 299)]
        );
        store.settle_range(
            &group,
            range(200, 299),
            vec![event(250, "a", 0)],
            true,
            LATER,
        );
        store.build_view(epoch, 100, 299);
        let rows = store.copy_rows(epoch, &[0, 1]).unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row.event.sequence)
                .collect::<Vec<_>>(),
            vec![0, 1],
            "the event fetched again keeps the number it had"
        );
    }

    #[test]
    fn the_view_holds_every_event_of_the_period_once_in_time_order() {
        // BR1.5: committed and staging of every chosen log group, FR2.7.
        let store = QueryStore::new();
        let (orders, payments) = (key("/orders"), key("/payments"));
        let epoch = store.begin_run(vec![orders.clone(), payments.clone()]);
        store.settle_range(
            &orders,
            range(100, 199),
            vec![event(100, "s1", 0), event(190, "s1", 1)],
            true,
            LATER,
        );
        store.settle_range(
            &orders,
            range(200, 299),
            vec![event(200, "s1", 0)],
            false,
            LATER,
        );
        store.settle_range(
            &payments,
            range(100, 299),
            vec![
                event(150, "s1", 0),
                event(200, "s1", 1),
                event(290, "s1", 2),
            ],
            true,
            LATER,
        );
        assert_eq!(store.build_view(epoch, 150, 289), Some(4));
        assert_eq!(
            view(&store, epoch),
            vec![
                row(150, "/payments", "s1"),
                row(190, "/orders", "s1"),
                row(200, "/orders", "s1"),
                row(200, "/payments", "s1"),
            ],
            "only the period; equal times by log group"
        );
        assert_eq!(store.view_len(epoch + 1), None);
        assert_eq!(
            store.copy_rows(epoch + 1, &[0]),
            None,
            "another epoch reads nothing"
        );
    }

    /// Runs `keys` and settles `[100, 199]` with one event for each of them.
    fn run_and_settle(store: &QueryStore, keys: &[HoldingKey]) -> u64 {
        let epoch = store.begin_run(keys.to_vec());
        for group in keys {
            store.settle_range(
                group,
                range(100, 199),
                vec![event(100, "a", 0)],
                true,
                LATER,
            );
        }
        epoch
    }

    #[test]
    fn a_log_group_no_longer_chosen_is_discarded_at_the_next_run() {
        // 261020-carryover-fixes FR2.2.1, FR2.2.3, FR2.2.4 (BR1.3, BR1.6).
        let store = QueryStore::new();
        let (orders, payments) = (key("/orders"), key("/payments"));
        run_and_settle(&store, &[orders.clone(), payments.clone()]);
        let epoch = store.begin_run(vec![orders.clone()]);
        assert_eq!(store.counts(&payments), (0, 0), "discarded");
        assert!(store.covered_ranges(&payments).is_empty());
        assert_eq!(store.counts(&orders), (1, 0), "still chosen: kept");
        assert_eq!(store.covered_ranges(&orders), vec![range(100, 199)]);
        assert_eq!(store.len(), 1);
        store.build_view(epoch, 0, 1_000);
        assert_eq!(view(&store, epoch), vec![row(100, "/orders", "a")]);
        // A connection change still discards everything.
        let after = store.discard_all();
        assert_eq!(after, epoch + 1);
        assert_eq!(store.counts(&orders), (0, 0));
        assert!(store.covered_ranges(&orders).is_empty());
        assert!(store.is_empty());
        assert_eq!(store.view_len(epoch), None);
    }

    #[test]
    fn a_log_group_chosen_again_is_fetched_again_while_the_others_keep_their_ranges() {
        // 261020-carryover-fixes FR2.2.2, FR2.2.3.
        let store = QueryStore::new();
        let (orders, payments) = (key("/orders"), key("/payments"));
        run_and_settle(&store, &[orders.clone(), payments.clone()]);
        store.begin_run(vec![orders.clone()]);
        store.begin_run(vec![orders.clone(), payments.clone()]);
        assert_eq!(
            store.missing_ranges(&payments, 100, 299),
            vec![range(100, 299)],
            "the whole period is fetched again"
        );
        assert_eq!(
            store.missing_ranges(&orders, 100, 299),
            vec![range(200, 299)],
            "only the part not held"
        );
        assert_eq!(store.counts(&payments), (0, 0));
        assert_eq!(store.counts(&orders), (1, 0));
    }

    #[test]
    fn cached_events_are_committed_and_their_ranges_covered() {
        // BR2.1.
        let store = QueryStore::new();
        let group = key("/g");
        let epoch = store.begin_run(vec![group.clone()]);
        let cached = |timestamp, sequence| CachedEvent {
            timestamp,
            ingestion_time: None,
            message: "from the cache".to_string(),
            log_stream_name: "a".to_string(),
            sequence,
        };
        store.add_cached(
            &group,
            &[range(100, 149)],
            vec![cached(100, 5), cached(149, 6), cached(150, 7)],
        );
        assert_eq!(store.counts(&group), (2, 0), "only events of the ranges");
        assert_eq!(
            store.missing_ranges(&group, 100, 199),
            vec![range(150, 199)]
        );
        store.settle_range(
            &group,
            range(150, 199),
            vec![event(160, "a", 0)],
            true,
            LATER,
        );
        assert_eq!(
            store.covered_ranges(&group),
            vec![range(100, 199)],
            "joined"
        );
        store.build_view(epoch, 100, 199);
        let rows = store.copy_rows(epoch, &[0, 1, 2]).unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row.event.sequence)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn evaluation_reads_the_view_and_stops_on_a_new_epoch() {
        let store = QueryStore::new();
        let group = key("/aws/lambda/orders");
        let epoch = store.begin_run(vec![group.clone()]);
        store.settle_range(
            &group,
            range(0, 10),
            vec![event(1, "a", 0), event(2, "b", 0), event(3, "a", 1)],
            true,
            LATER,
        );
        assert_eq!(store.build_view(epoch, 0, 10), Some(3));
        let plan = parse_query("filter @log = '/aws/lambda/orders' and @logStream = 'a'").unwrap();
        let mut evaluation = Evaluation::new(&plan, 3);
        assert!(store.evaluate_step(&plan, &mut evaluation, epoch));
        assert!(evaluation.is_done());
        assert_eq!(
            evaluation.into_output().map(|output| output.rows),
            Some(vec![2, 0]),
            "newest first (U2:BR4.4)"
        );
        let mut late = Evaluation::new(&plan, 3);
        store.begin_run(vec![group]);
        assert!(!store.evaluate_step(&plan, &mut late, epoch));
        assert_eq!(late.evaluated(), 0, "nothing was evaluated");
        assert_eq!(store.build_view(epoch, 0, 10), None);
    }

    #[test]
    fn json_field_names_come_from_the_view_of_the_epoch() {
        // U3:BR3.3 (U2:BR1.7).
        let store = QueryStore::new();
        let group = key("/g");
        let epoch = store.begin_run(vec![group.clone()]);
        let mut json = event(1, "a", 0);
        json.message = r#"{"level":"error","http":{"status":500}}"#.to_string();
        let mut outside = event(50, "a", 1);
        outside.message = r#"{"outside":1}"#.to_string();
        store.settle_range(
            &group,
            range(0, 100),
            vec![json, event(2, "a", 1), outside],
            true,
            LATER,
        );
        store.build_view(epoch, 0, 10);
        assert_eq!(
            store.json_field_names(epoch),
            Some(vec!["http.status".to_string(), "level".to_string()])
        );
        assert_eq!(store.json_field_names(epoch + 1), None);
    }

    #[test]
    fn the_holdings_are_not_the_log_management_timeline() {
        // U1:BR1.7: filling the query holdings leaves the Log Management
        // screen's LogView as it was, and the other way round.
        use crate::coordinator::TimelineStore;
        let log_view = Mutex::new(LogView::new());
        log_view.add(vec![event(5, "lm", 0)]);
        let store = QueryStore::new();
        let group = key("/g");
        store.begin_run(vec![group.clone()]);
        store.settle_range(
            &group,
            range(0, 9),
            vec![event(1, "q", 0), event(2, "q", 1)],
            true,
            LATER,
        );
        assert_eq!(log_view.lock().unwrap().rows(0, 10).all_count, 1);
        log_view.discard();
        assert_eq!(store.len(), 2);
    }
}

/// 261017-insights-query U3:BR5.3, NFR2, functional-design review R-03:
/// the peak memory and the evaluation time with both screens full: Log
/// Management holding 1,000,000 events, the query screen 1,000,000 more
/// over 50 log groups, a previous result of 100,000 copied rows, then a
/// query with a `sort` on a JSON field evaluated over the whole view. Run
/// with `cargo test --release -p local-sights-core query:: -- --ignored
/// --nocapture`. No memory limit is set (BR5.3); the test checks that the
/// evaluation ends with a consistent result and prints the numbers. The
/// peak resident size is the OS's own (`VmHWM` on Linux); on macOS run the
/// command under `/usr/bin/time -l` for its "maximum resident set size".
#[cfg(test)]
mod memory {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::coordinator::TimelineStore;
    use crate::log_view::LogView;
    use crate::query::parser::parse_query;

    const EVENTS: usize = 1_000_000;
    const GROUPS: usize = 50;

    /// The process's peak resident size in KiB, where the OS tells it.
    fn peak_resident_kib() -> Option<u64> {
        if cfg!(target_os = "linux") {
            let status = std::fs::read_to_string("/proc/self/status").ok()?;
            let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
            return line.split_whitespace().nth(1)?.parse().ok();
        }
        None
    }

    fn peak_text() -> String {
        peak_resident_kib().map_or_else(
            || "not told by this OS".to_string(),
            |kib| format!("{:.1} MiB", kib as f64 / 1024.0),
        )
    }

    /// `count` events in time order across 8 streams, as the evaluator's
    /// speed tests make them: nine in ten JSON objects with a level, a
    /// nested status and a latency, one in ten a plain text line.
    fn events(count: usize, from: usize) -> Vec<LogEvent> {
        (from..from + count)
            .map(|index| {
                let level = if index % 7 == 0 { "ERROR" } else { "INFO" };
                let status = if index % 11 == 0 { 503 } else { 200 };
                let latency = (index * 7919) % 100_003;
                let message = if index % 10 == 9 {
                    format!("plain text line {index} {level} without JSON")
                } else {
                    format!(
                        "{{\"level\":\"{level}\",\"http\":{{\"status\":{status},\"path\":\"/orders/{index}\"}},\"latency\":{latency},\"msg\":\"request {index}\"}}"
                    )
                };
                LogEvent {
                    timestamp: 1_756_000_000_000 + i64::try_from(index / 3).unwrap_or(i64::MAX),
                    ingestion_time: None,
                    message,
                    log_stream_name: format!("stream-{}", index % 8),
                    sequence: 0,
                }
            })
            .collect()
    }

    fn key(index: usize) -> HoldingKey {
        HoldingKey {
            profile: ProfileSelector::Named("dev".to_string()),
            region: "ap-northeast-1".to_string(),
            log_group_name: format!("/aws/lambda/service-{index:02}"),
        }
    }

    /// Evaluates `text` over the view of `epoch` step by step, as the runner
    /// does, and copies its rows; returns the copies, the evaluation time
    /// and the longest step.
    fn evaluate(
        store: &QueryStore,
        epoch: u64,
        text: &str,
    ) -> (Vec<ResultRow>, Duration, Duration) {
        let plan = parse_query(text).unwrap();
        let total = store.view_len(epoch).unwrap();
        let mut evaluation = Evaluation::new(&plan, total);
        let started = Instant::now();
        let mut longest = Duration::ZERO;
        while !evaluation.is_done() {
            let before = Instant::now();
            assert!(store.evaluate_step(&plan, &mut evaluation, epoch));
            longest = longest.max(before.elapsed());
        }
        let output = evaluation.into_output().unwrap();
        let elapsed = started.elapsed();
        assert_eq!(output.evaluated_count, total);
        let rows = store.copy_rows(epoch, &output.rows).unwrap();
        (rows, elapsed, longest)
    }

    #[test]
    #[ignore = "measures memory and speed; run with --release --ignored"]
    fn both_screens_full_and_a_previous_result_evaluate_a_json_sort() {
        println!("peak at the start: {}", peak_text());
        let log_view = Mutex::new(LogView::new());
        for page in events(EVENTS, 0).chunks(10_000) {
            log_view.add(page.to_vec());
        }
        assert_eq!(
            log_view.lock().unwrap().rows(0, 1).all_count as usize,
            EVENTS
        );
        println!(
            "peak with Log Management's 1,000,000 events: {}",
            peak_text()
        );

        let store = QueryStore::new();
        let keys: Vec<HoldingKey> = (0..GROUPS).map(key).collect();
        let epoch = store.begin_run(keys.clone());
        let per_group = EVENTS / GROUPS;
        let range = CoveredRange {
            start_ms: 1_756_000_000_000,
            end_ms: 1_756_000_000_000 + i64::try_from(EVENTS).unwrap_or(i64::MAX),
        };
        let started = Instant::now();
        for (index, group) in keys.iter().enumerate() {
            store.settle_range(
                group,
                range,
                events(per_group, index * per_group),
                true,
                i64::MAX,
            );
        }
        println!(
            "settling 50 log groups of 20,000 events: {:?}",
            started.elapsed()
        );
        let started = Instant::now();
        let length = store
            .build_view(epoch, range.start_ms, range.end_ms)
            .unwrap();
        assert_eq!(length, EVENTS);
        println!(
            "building the view of 1,000,000 events: {:?}",
            started.elapsed()
        );
        println!(
            "peak with the query screen's 1,000,000 events too: {}",
            peak_text()
        );

        let (previous, elapsed, _) = evaluate(&store, epoch, "sort latency desc | limit 100000");
        assert_eq!(previous.len(), 100_000);
        println!(
            "the previous result, 100,000 copied rows, in {elapsed:?}; peak: {}",
            peak_text()
        );

        let query = "fields @timestamp, @log, level, latency | filter level = 'INFO' | sort latency desc, @timestamp | limit 100000";
        let (rows, elapsed, longest) = evaluate(&store, epoch, query);
        assert_eq!(rows.len(), 100_000);
        assert!(
            rows.windows(2).all(|pair| {
                let latency = |row: &ResultRow| {
                    serde_json::from_str::<serde_json::Value>(&row.event.message)
                        .ok()
                        .and_then(|value| value["latency"].as_u64())
                };
                latency(&pair[0]) >= latency(&pair[1])
            }),
            "highest latency first"
        );
        println!(
            "the JSON sort query over 1,000,000 events of 50 log groups: {elapsed:?} (longest step {longest:?}); peak: {}",
            peak_text()
        );
        drop(previous);
    }
}
