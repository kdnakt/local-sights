//! QueryStore, the minimal version (261017-insights-query U1:BR2.1, BR2.2,
//! BR7.2, entities.md QueryHoldings).
//!
//! The query screen's own holdings: the events of one log group, fetched
//! for the last run. They are not the Log Management screen's EventTimeline
//! and nothing is passed between the two (FR2.1, RA Q3b, U1:BR1.7). Every run
//! discards them and fetches the whole range again (units Q2a); the fetched
//! range is not remembered until U3.
//!
//! The store is a [`TimelineStore`] and [`HeldEvents`], so the existing
//! fetch flow (`run_fetch_with_cache`) can fill it unchanged (U1:BR2.2). Each
//! call locks it once and releases it, so the screen can read result rows
//! while a fetch or an evaluation runs. Every discard increments the epoch;
//! a result knows the epoch it points into, and rows are read only while the
//! epoch is the same (U1:BR6.4, functional-design review R-05).

use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::cache::plan::CoveredRange;
use crate::catalog::ProfileSelector;
use crate::coordinator::{HeldEvents, TimelineStore, copy_chunk_in_range};
use crate::event::LogEvent;
use crate::query::evaluator::{Evaluation, GroupEvents};
use crate::query::fields::{FIELD_NAME_SCAN_EVENTS, collect_json_field_names};
use crate::query::plan::QueryPlan;
use crate::timeline::EventTimeline;

/// What the holdings were fetched for (entities.md HoldingKey).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldingKey {
    /// The profile of the connection.
    pub profile: ProfileSelector,
    /// The region of the connection.
    pub region: String,
    /// The log group, the value of `@log` (U1:BR4.1).
    pub log_group_name: String,
}

/// The holdings behind the lock.
#[derive(Debug, Default)]
struct QueryHoldings {
    key: Option<HoldingKey>,
    timeline: EventTimeline,
    epoch: u64,
}

impl QueryHoldings {
    fn discard(&mut self) -> u64 {
        self.timeline.clear();
        self.epoch += 1;
        self.epoch
    }
}

/// Events fetched by the query screen, with their epoch.
#[derive(Debug, Default)]
pub struct QueryStore {
    holdings: Mutex<QueryHoldings>,
}

/// Events copied out of the store with the log group they belong to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldRows {
    /// The log group of every event (`@log`).
    pub log_group_name: String,
    /// The events, in the order asked.
    pub events: Vec<LogEvent>,
}

impl QueryStore {
    /// Empty holdings at epoch 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// A poisoned lock still holds consistent holdings (every change is one
    /// step), so it is recovered.
    fn lock(&self) -> MutexGuard<'_, QueryHoldings> {
        self.holdings.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// U1:BR2.1: before a run's fetch, discards the holdings and names what
    /// the new ones are for. Returns the new epoch.
    pub fn begin_run(&self, key: HoldingKey) -> u64 {
        let mut holdings = self.lock();
        holdings.key = Some(key);
        holdings.discard()
    }

    /// U1:BR7.2: the connection changed; everything goes, the key too.
    /// Returns the new epoch.
    pub fn discard_all(&self) -> u64 {
        let mut holdings = self.lock();
        holdings.key = None;
        holdings.discard()
    }

    /// The current epoch.
    pub fn epoch(&self) -> u64 {
        self.lock().epoch
    }

    /// What the holdings were fetched for, if anything.
    pub fn key(&self) -> Option<HoldingKey> {
        self.lock().key.clone()
    }

    /// Number of held events.
    pub fn len(&self) -> usize {
        self.lock().timeline.len()
    }

    /// Whether no event is held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// U1:BR5.8, U2:BR3.1: does the next step of `evaluation` (one chunk of
    /// work and what follows it without one) under one lock, lending the
    /// held events with their log group name in time order. Returns
    /// `false`, doing nothing, when the holdings are no longer those of
    /// `epoch` (a newer run or a connection change discarded them).
    pub fn evaluate_step(&self, plan: &QueryPlan, evaluation: &mut Evaluation, epoch: u64) -> bool {
        let holdings = self.lock();
        if holdings.epoch != epoch {
            return false;
        }
        let items = GroupEvents {
            events: holdings.timeline.events(),
            log_group_name: holdings
                .key
                .as_ref()
                .map_or("", |key| key.log_group_name.as_str()),
        };
        evaluation.step(plan, &items);
        true
    }

    /// U2:BR1.7: the JSON field names of the first
    /// [`FIELD_NAME_SCAN_EVENTS`] held events, when the holdings are still
    /// those of `epoch`.
    pub fn json_field_names(&self, epoch: u64) -> Option<Vec<String>> {
        let holdings = self.lock();
        if holdings.epoch != epoch {
            return None;
        }
        let messages = holdings
            .timeline
            .events()
            .iter()
            .take(FIELD_NAME_SCAN_EVENTS)
            .map(|event| event.message.as_str());
        Some(collect_json_field_names(messages))
    }

    /// The events at `positions` (R-05: positions in the holdings), when the
    /// holdings are still those of `epoch`; positions beyond the end are
    /// skipped. `None` when the epoch changed (U1:BR6.4).
    pub fn rows_at(&self, epoch: u64, positions: &[usize]) -> Option<HeldRows> {
        let holdings = self.lock();
        if holdings.epoch != epoch {
            return None;
        }
        let events = holdings.timeline.events();
        Some(HeldRows {
            log_group_name: holdings
                .key
                .as_ref()
                .map(|key| key.log_group_name.clone())
                .unwrap_or_default(),
            events: positions
                .iter()
                .filter_map(|position| events.get(*position).cloned())
                .collect(),
        })
    }
}

impl TimelineStore for QueryStore {
    /// The fetch's own discard at its start, or its discard when the app
    /// closes (Aborted): the key stays, the epoch goes up.
    fn discard(&self) -> u64 {
        let mut holdings = self.lock();
        holdings.discard();
        holdings.timeline.version()
    }

    fn add(&self, events: Vec<LogEvent>) -> u64 {
        self.lock().timeline.append(events)
    }
}

impl HeldEvents for QueryStore {
    fn copy_events_in_range(
        &self,
        range: CoveredRange,
        cursor: Option<usize>,
        max: usize,
    ) -> (Vec<LogEvent>, Option<usize>) {
        let holdings = self.lock();
        copy_chunk_in_range(holdings.timeline.events(), range, cursor, max)
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

    fn stamps(rows: &HeldRows) -> Vec<i64> {
        rows.events.iter().map(|event| event.timestamp).collect()
    }

    #[test]
    fn every_discard_increments_the_epoch_and_empties_the_holdings() {
        let store = QueryStore::new();
        assert_eq!(store.epoch(), 0);
        assert_eq!(store.begin_run(key("/g")), 1);
        store.add(vec![event(1, "s", 0)]);
        assert_eq!(store.len(), 1);
        store.discard();
        assert_eq!(store.epoch(), 2, "the fetch's own discard counts too");
        assert!(store.is_empty());
        assert_eq!(store.key(), Some(key("/g")), "and keeps the key");
        assert_eq!(store.begin_run(key("/other")), 3);
        assert_eq!(store.key(), Some(key("/other")));
    }

    #[test]
    fn pages_are_held_in_time_order_across_streams() {
        let store = QueryStore::new();
        let epoch = store.begin_run(key("/g"));
        store.add(vec![event(30, "b", 0), event(10, "b", 1)]);
        store.add(vec![event(20, "a", 0)]);
        let rows = store.rows_at(epoch, &[0, 1, 2]).unwrap();
        assert_eq!(stamps(&rows), vec![10, 20, 30]);
        assert_eq!(rows.log_group_name, "/g");
        let picked = store.rows_at(epoch, &[2, 0, 9]).unwrap();
        assert_eq!(
            stamps(&picked),
            vec![30, 10],
            "in the order asked, past the end skipped"
        );
        assert_eq!(
            store.rows_at(epoch + 1, &[0]),
            None,
            "another epoch reads nothing"
        );
    }

    #[test]
    fn the_cache_write_copies_the_events_of_a_range_chunk_by_chunk() {
        let store = QueryStore::new();
        store.begin_run(key("/g"));
        store.add((0..10).map(|i| event(i * 10, "s", i as u64)).collect());
        let range = CoveredRange {
            start_ms: 20,
            end_ms: 70,
        };
        let (first, next) = store.copy_events_in_range(range, None, 4);
        assert_eq!(
            first.iter().map(|e| e.timestamp).collect::<Vec<_>>(),
            vec![20, 30, 40, 50]
        );
        let (rest, done) = store.copy_events_in_range(range, next, 4);
        assert_eq!(
            rest.iter().map(|e| e.timestamp).collect::<Vec<_>>(),
            vec![60, 70]
        );
        assert_eq!(done, None);
    }

    #[test]
    fn a_connection_change_discards_the_events_and_the_key() {
        let store = QueryStore::new();
        let epoch = store.begin_run(key("/g"));
        store.add(vec![event(1, "s", 0)]);
        let after = store.discard_all();
        assert_eq!(after, epoch + 1);
        assert_eq!(store.key(), None);
        assert!(store.is_empty());
        assert_eq!(
            store.rows_at(after, &[0]).map(|rows| rows.events.len()),
            Some(0)
        );
        assert_eq!(store.rows_at(after, &[0]).unwrap().log_group_name, "");
    }

    #[test]
    fn evaluation_reads_the_events_with_the_group_name_and_stops_on_a_new_epoch() {
        let store = QueryStore::new();
        let epoch = store.begin_run(key("/aws/lambda/orders"));
        store.add(vec![event(1, "a", 0), event(2, "b", 0), event(3, "a", 1)]);
        let plan = parse_query("filter @log = '/aws/lambda/orders' and @logStream = 'a'").unwrap();
        let mut evaluation = Evaluation::new(&plan, store.len());
        assert!(store.evaluate_step(&plan, &mut evaluation, epoch));
        assert_eq!(evaluation.evaluated(), 3);
        assert!(evaluation.is_done());
        assert_eq!(
            evaluation.into_output().map(|output| output.rows),
            Some(vec![2, 0]),
            "newest first (U2:BR4.4)"
        );

        let mut late = Evaluation::new(&plan, store.len());
        store.discard_all();
        assert!(!store.evaluate_step(&plan, &mut late, epoch));
        assert_eq!(late.evaluated(), 0, "nothing was evaluated");
    }

    #[test]
    fn json_field_names_come_from_the_holdings_of_the_epoch() {
        // U2:BR1.7.
        let store = QueryStore::new();
        let epoch = store.begin_run(key("/g"));
        let mut json = event(1, "a", 0);
        json.message = r#"{"level":"error","http":{"status":500}}"#.to_string();
        store.add(vec![json, event(2, "a", 1)]);
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
        let log_view = Mutex::new(LogView::new());
        log_view.add(vec![event(5, "lm", 0)]);
        let store = QueryStore::new();
        store.begin_run(key("/g"));
        store.add(vec![event(1, "q", 0), event(2, "q", 1)]);
        assert_eq!(log_view.lock().unwrap().rows(0, 10).all_count, 1);
        log_view.discard();
        assert_eq!(store.len(), 2);
    }
}
