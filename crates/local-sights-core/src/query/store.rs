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
//!
//! Since 261025-insights-stats U3 the holder of every range under fetch is
//! registered here ([`QueryStore::register_inflight`]) and settled with
//! [`QueryStore::settle_inflight`], which takes its events out, settles them
//! and unregisters it under one lock of the store, so no event is ever in
//! both its holder and the settled holdings, nor in neither (U3:BR1.2,
//! FR4.8). The lock order is fixed: the store, then a holder; the fetch
//! takes its holder's lock only. A partial view
//! ([`QueryStore::build_partial_view`]) is the settled events plus copies of
//! the events under fetch, numbered as settling will number them, in the
//! result's order (U3:BR1.3). Besides the epoch the view has a version,
//! moved on by every view built and every change of the holdings; a step or
//! a copy over a view whose `(epoch, version)` is no longer the store's does
//! nothing (U3:BR1.4).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

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

/// Where an event of a view is (entities.md ViewEntry; 261025-insights-stats
/// U3:BR1.3, FR review R-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewSource {
    /// The committed events of the log group.
    Committed,
    /// The staging events of the log group.
    Staging,
    /// A copy of an event under fetch, held by the view itself.
    Inflight,
}

impl From<Holding> for ViewSource {
    fn from(holding: Holding) -> Self {
        match holding {
            Holding::Committed => ViewSource::Committed,
            Holding::Staging => ViewSource::Staging,
        }
    }
}

/// One event of a run's view (entities.md ViewEntry): the chosen log group
/// it belongs to (by its place in the run's choice), its holding and its
/// place there. Valid while the view's [`ViewStamp`] is the store's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewEntry {
    /// The log group's place in the run's choice.
    pub group: usize,
    /// Committed, staging, or a copy of an event under fetch.
    pub source: ViewSource,
    /// The event's place in that holding.
    pub position: usize,
}

/// Which view a step or a copy is for (261025-insights-stats U3:BR1.4): the
/// epoch of its run and the version of the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewStamp {
    /// The run's epoch.
    pub epoch: u64,
    /// The view's version.
    pub version: u64,
}

/// What [`QueryStore::build_partial_view`] built (entities.md PartialView):
/// the view itself stays in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialView {
    /// Steps and copies over this view give it.
    pub stamp: ViewStamp,
    /// Events in the view.
    pub length: usize,
    /// Of those, copies of events under fetch.
    pub inflight_count: usize,
}

/// The number [`QueryStore::register_inflight`] gives a holder.
pub type InflightId = u64;

/// The holder of one range under fetch (entities.md InflightHolder).
#[derive(Debug)]
struct InflightHolder {
    id: InflightId,
    key: HoldingKey,
    range: CoveredRange,
    timeline: Arc<Mutex<EventTimeline>>,
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
    /// 261025-insights-stats U3:BR1.4: moved on by every view built and
    /// every change of the holdings.
    view_version: u64,
    /// The copies of the events under fetch the view holds (U3:BR1.3).
    inflight_copies: Vec<LogEvent>,
    /// The holders of the ranges under fetch (entities.md
    /// InflightRegistry).
    inflight: Vec<InflightHolder>,
    next_inflight: InflightId,
}

impl Holdings {
    /// U3:BR1.4: the holdings changed, or a view was built.
    fn move_view_on(&mut self) {
        self.view_version += 1;
    }

    /// Whether `stamp` is the current view's.
    fn is_current(&self, stamp: ViewStamp) -> bool {
        self.epoch == stamp.epoch
            && self.view_epoch == Some(stamp.epoch)
            && self.view_version == stamp.version
    }

    /// The settled events of every chosen log group in `[start_ms,
    /// end_ms]`, not yet in order.
    fn settled_entries(&self, start_ms: i64, end_ms: i64) -> Vec<ViewEntry> {
        let mut view = Vec::new();
        for (group_index, key) in self.run_keys.iter().enumerate() {
            let Some(group) = self.groups.get(key) else {
                continue;
            };
            for source in [Holding::Committed, Holding::Staging] {
                let events = group.holding(source);
                let first = events.partition_point(|event| event.timestamp < start_ms);
                let last = events.partition_point(|event| event.timestamp <= end_ms);
                view.extend((first..last.max(first)).map(|position| ViewEntry {
                    group: group_index,
                    source: source.into(),
                    position,
                }));
            }
        }
        view
    }

    /// Puts `view` in `(timestamp, log group, stream, order)` order and
    /// makes it the view of `epoch` (BR1.5, U3:BR1.3), moving its version
    /// on. Returns the stamp.
    fn install_view(&mut self, mut view: Vec<ViewEntry>, epoch: u64) -> ViewStamp {
        {
            let items = RunItems::of(self);
            view.sort_by(
                |left, right| match (items.entry(*left), items.entry(*right)) {
                    (Some(left), Some(right)) => order_key(&left).cmp(&order_key(&right)),
                    _ => std::cmp::Ordering::Equal,
                },
            );
        }
        self.view = view;
        self.view_epoch = Some(epoch);
        self.move_view_on();
        ViewStamp {
            epoch,
            version: self.view_version,
        }
    }

    /// U3:BR1.2: settles `events` of `range` of `key` (see
    /// [`QueryStore::settle_range`]).
    fn settle_events(
        &mut self,
        key: &HoldingKey,
        range: CoveredRange,
        events: Vec<LogEvent>,
        (completed, run_started_at_ms): (bool, i64),
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
        let group = self.groups.entry(key.clone()).or_default();
        group.add(taken, Holding::Committed);
        if let Some(part) = committed {
            group.covered = merge_ranges(&group.covered, part);
        }
        group.add(staged, Holding::Staging);
        self.move_view_on();
        committed
    }

    /// U3:BR1.3: copies the events of the holder at `index` of the registry
    /// that lie in its range, numbered as settling will number them (from
    /// the stream series of its log group, which does not move), and keeps
    /// those in `[start_ms, end_ms]` as copies of the view. Returns their
    /// entries. The holder's lock is taken briefly, after the store's.
    fn copy_inflight(&mut self, index: usize, start_ms: i64, end_ms: i64) -> Vec<ViewEntry> {
        let Some(holder) = self.inflight.get(index) else {
            return Vec::new();
        };
        let Some(group_index) = self.run_keys.iter().position(|key| *key == holder.key) else {
            return Vec::new();
        };
        // Step 22: in chunks, so the fetch never waits long for its holder.
        let fetched = copy_in_chunks(&holder.timeline, holder.range, INFLIGHT_COPY_CHUNK);
        let mut sequences = self
            .groups
            .get(&holder.key)
            .map(|group| group.sequences.clone())
            .unwrap_or_default();
        let numbered = sequences.number(fetched, Holding::Staging);
        let mut entries = Vec::new();
        for event in numbered {
            if event.timestamp < start_ms || event.timestamp > end_ms {
                continue;
            }
            entries.push(ViewEntry {
                group: group_index,
                source: ViewSource::Inflight,
                position: self.inflight_copies.len(),
            });
            self.inflight_copies.push(event);
        }
        entries
    }
}

/// 261025-insights-stats U3 (functional-spec 8, Step 22): the most events
/// a partial view copies from a holder under one lock of it. Measured on a
/// release build, one lock over 300,000 events took about 58 ms and over
/// 500,000 about 78 ms, longer than the 50 ms the fetch may wait for its
/// holder; 50,000 keeps one lock at about 8 ms.
const INFLIGHT_COPY_CHUNK: usize = 50_000;

/// One chunk of a holder's copy, under one lock of it: the events of `range`
/// from the time `cursor` on (from its start without one), at most `chunk`
/// of them, then every other event of the last time copied, so equal times
/// are never split. Returns the copies and the time the next chunk starts
/// from, `None` when the range is done. A page added between two chunks
/// before the time reached is left for the next partial view: no event is
/// copied twice and the copies stay in the timeline's order.
fn copy_chunk(
    timeline: &Mutex<EventTimeline>,
    range: CoveredRange,
    cursor: Option<i64>,
    chunk: usize,
) -> (Vec<LogEvent>, Option<i64>) {
    let timeline = timeline.lock().unwrap_or_else(PoisonError::into_inner);
    let events = timeline.events();
    let from = cursor.map_or(range.start_ms, |at| at.max(range.start_ms));
    let first = events.partition_point(|event| event.timestamp < from);
    let end = events.partition_point(|event| event.timestamp <= range.end_ms);
    if first >= end {
        return (Vec::new(), None);
    }
    let reach = first.saturating_add(chunk.max(1)).min(end);
    let Some(edge) = reach.checked_sub(1).and_then(|last| events.get(last)) else {
        return (Vec::new(), None);
    };
    let edge_time = edge.timestamp;
    let last = events
        .partition_point(|event| event.timestamp <= edge_time)
        .min(end);
    let copies = events
        .get(first..last)
        .map(<[LogEvent]>::to_vec)
        .unwrap_or_default();
    let next = (last < end).then(|| edge_time.saturating_add(1));
    (copies, next)
}

/// The events of `range` in `timeline`, copied `chunk` at a time, each under
/// a lock of its own ([`copy_chunk`]).
fn copy_in_chunks(
    timeline: &Mutex<EventTimeline>,
    range: CoveredRange,
    chunk: usize,
) -> Vec<LogEvent> {
    let mut copies = Vec::new();
    let mut cursor = None;
    loop {
        let (part, next) = copy_chunk(timeline, range, cursor, chunk);
        copies.extend(part);
        match next {
            Some(at) => cursor = Some(at),
            None => return copies,
        }
    }
}

/// Where an event stands in a view: `(timestamp, log group, stream, order)`.
fn order_key<'a>(item: &EvalItem<'a>) -> (i64, &'a str, &'a str, u64) {
    (
        item.event.timestamp,
        item.log_group_name,
        item.event.log_stream_name.as_str(),
        item.event.sequence,
    )
}

/// The events of a run's view, lent to the evaluator by position.
struct RunItems<'a> {
    view: &'a [ViewEntry],
    groups: Vec<Option<(&'a GroupHoldings, &'a str)>>,
    names: Vec<&'a str>,
    copies: &'a [LogEvent],
}

impl<'a> RunItems<'a> {
    fn of(holdings: &'a Holdings) -> Self {
        Self {
            view: &holdings.view,
            groups: holdings
                .run_keys
                .iter()
                .map(|key| {
                    holdings
                        .groups
                        .get(key)
                        .map(|group| (group, key.log_group_name.as_str()))
                })
                .collect(),
            names: holdings
                .run_keys
                .iter()
                .map(|key| key.log_group_name.as_str())
                .collect(),
            copies: &holdings.inflight_copies,
        }
    }

    fn entry(&self, entry: ViewEntry) -> Option<EvalItem<'a>> {
        let (event, log_group_name) = match entry.source {
            ViewSource::Inflight => (
                self.copies.get(entry.position)?,
                *self.names.get(entry.group)?,
            ),
            ViewSource::Committed | ViewSource::Staging => {
                let (group, log_group_name) = (*self.groups.get(entry.group)?)?;
                let holding = match entry.source {
                    ViewSource::Committed => Holding::Committed,
                    _ => Holding::Staging,
                };
                (group.holding(holding).get(entry.position)?, log_group_name)
            }
        };
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
        Self::forget_view(&mut holdings);
        holdings.epoch += 1;
        holdings.epoch
    }

    /// The view, its copies and the holders under fetch go (a run starts or
    /// the connection changes, 261025-insights-stats U3:BR1.2, BR1.4).
    fn forget_view(holdings: &mut Holdings) {
        holdings.view = Vec::new();
        holdings.view_epoch = None;
        holdings.inflight_copies = Vec::new();
        holdings.inflight.clear();
        holdings.move_view_on();
    }

    /// U3:BR1.6: the connection changed; every log group's holdings and
    /// ranges go. Returns the new epoch.
    pub fn discard_all(&self) -> u64 {
        let mut holdings = self.lock();
        holdings.groups.clear();
        holdings.run_keys.clear();
        Self::forget_view(&mut holdings);
        holdings.epoch += 1;
        holdings.epoch
    }

    /// The view's version (261025-insights-stats U3:BR1.4).
    pub fn view_version(&self) -> u64 {
        self.lock().view_version
    }

    /// The stamp of the current view when it is the view of `epoch`.
    pub fn view_stamp(&self, epoch: u64) -> Option<ViewStamp> {
        let holdings = self.lock();
        (holdings.epoch == epoch && holdings.view_epoch == Some(epoch)).then_some(ViewStamp {
            epoch,
            version: holdings.view_version,
        })
    }

    /// 261025-insights-stats U3:BR1.2: the fetch of `range` of `key` starts
    /// into `timeline`, its own holder, which the store keeps until
    /// [`QueryStore::settle_inflight`]. The fetch itself only ever locks
    /// its holder (the lock order is the store, then a holder).
    pub fn register_inflight(
        &self,
        key: &HoldingKey,
        range: CoveredRange,
        timeline: Arc<Mutex<EventTimeline>>,
    ) -> InflightId {
        let mut holdings = self.lock();
        holdings.next_inflight += 1;
        let id = holdings.next_inflight;
        holdings.inflight.push(InflightHolder {
            id,
            key: key.clone(),
            range,
            timeline,
        });
        id
    }

    /// Number of ranges under fetch registered.
    pub fn inflight_count(&self) -> usize {
        self.lock().inflight.len()
    }

    /// 261025-insights-stats U3:BR1.2, BR3.6, FR4.8, FR5.3: the fetch of
    /// holder `id` ended. Under one lock of the store its events are taken
    /// out of the holder, settled as [`QueryStore::settle_range`] settles
    /// them (staging only when not `completed`) and the holder is
    /// unregistered. Returns the range now covered; `None` as well when the
    /// holder is not registered (a new run or a connection change since).
    pub fn settle_inflight(
        &self,
        id: InflightId,
        completed: bool,
        run_started_at_ms: i64,
    ) -> Option<CoveredRange> {
        let mut holdings = self.lock();
        let index = holdings
            .inflight
            .iter()
            .position(|holder| holder.id == id)?;
        let holder = holdings.inflight.remove(index);
        let events = holder
            .timeline
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take_events();
        holdings.settle_events(
            &holder.key,
            holder.range,
            events,
            (completed, run_started_at_ms),
        )
    }

    /// 261025-insights-stats U3:BR1.3, BR1.4: builds the partial view of
    /// the run of `epoch`: the settled events of every chosen log group in
    /// `[start_ms, end_ms]` and copies of the events under fetch in that
    /// period, in the result's order. `None` when the epoch is no longer the
    /// run's.
    pub fn build_partial_view(
        &self,
        epoch: u64,
        start_ms: i64,
        end_ms: i64,
    ) -> Option<PartialView> {
        let mut holdings = self.lock();
        if holdings.epoch != epoch {
            return None;
        }
        holdings.inflight_copies = Vec::new();
        let mut view = holdings.settled_entries(start_ms, end_ms);
        let mut inflight_count = 0;
        for index in 0..holdings.inflight.len() {
            let copies = holdings.copy_inflight(index, start_ms, end_ms);
            inflight_count += copies.len();
            view.extend(copies);
        }
        let length = view.len();
        let stamp = holdings.install_view(view, epoch);
        Some(PartialView {
            stamp,
            length,
            inflight_count,
        })
    }

    /// U3:BR1.4: the next step of `evaluation` over the view stamped
    /// `stamp`, under one lock. Returns `false`, doing nothing, when that
    /// view is no longer the store's.
    pub fn evaluate_view_step(
        &self,
        plan: &QueryPlan,
        evaluation: &mut Evaluation,
        stamp: ViewStamp,
    ) -> bool {
        let holdings = self.lock();
        if !holdings.is_current(stamp) {
            return false;
        }
        evaluation.step(plan, &RunItems::of(&holdings));
        true
    }

    /// U3:BR1.4: as [`QueryStore::copy_rows`] over the view stamped
    /// `stamp`; `None` when that view is no longer the store's.
    pub fn copy_view_rows(&self, stamp: ViewStamp, positions: &[usize]) -> Option<Vec<ResultRow>> {
        let holdings = self.lock();
        if !holdings.is_current(stamp) {
            return None;
        }
        Some(copy_items(&RunItems::of(&holdings), positions))
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
        holdings.move_view_on();
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
        self.lock()
            .settle_events(key, range, events, (completed, run_started_at_ms))
    }

    /// U3:BR1.5: builds the view of the run of `epoch`: of every chosen log
    /// group, the committed and staging events in `[start_ms, end_ms]`, in
    /// `(timestamp, log group, stream, order)` order. Returns its length, or
    /// `None` when the epoch is no longer the run's. The copies of a partial
    /// view go (261025-insights-stats U3:BR1.3).
    pub fn build_view(&self, epoch: u64, start_ms: i64, end_ms: i64) -> Option<usize> {
        let mut holdings = self.lock();
        if holdings.epoch != epoch {
            return None;
        }
        holdings.inflight_copies = Vec::new();
        // Each holding is in time order already: the sort merges the runs.
        let view = holdings.settled_entries(start_ms, end_ms);
        let length = view.len();
        holdings.install_view(view, epoch);
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
        Some(copy_items(&RunItems::of(&holdings), positions))
    }
}

/// Copies the events at `positions` of `items` with their log group, in the
/// order asked (U3:BR3.2).
fn copy_items(items: &RunItems<'_>, positions: &[usize]) -> Vec<ResultRow> {
    positions
        .iter()
        .filter_map(|position| items.item(*position))
        .map(|item| ResultRow::Event {
            event: item.event.clone(),
            log_group_name: item.log_group_name.to_string(),
        })
        .collect()
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
            .filter_map(|row| match row {
                ResultRow::Event {
                    event,
                    log_group_name,
                } => Some((event.timestamp, log_group_name, event.log_stream_name)),
                ResultRow::Aggregate { .. } => None,
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
                .filter_map(|row| row.event().map(|event| event.sequence))
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
                .filter_map(|row| row.event().map(|event| event.sequence))
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
            evaluation.into_output().and_then(|output| match output {
                crate::query::evaluator::EvalOutput::Events(events) => Some(events.rows),
                crate::query::evaluator::EvalOutput::Aggregates(_) => None,
            }),
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

    // ---- 261025-insights-stats U3:BR1.2-BR1.4, BR2.2 (test first) ------

    use crate::query::evaluator::EvalOutput;
    use std::sync::Arc;

    /// A holder of a range under fetch, holding `events` (as the fetch
    /// numbers them, from 0 per stream).
    fn holder(events: Vec<LogEvent>) -> Arc<Mutex<EventTimeline>> {
        let mut timeline = EventTimeline::new();
        timeline.append(events);
        Arc::new(Mutex::new(timeline))
    }

    /// The `(timestamp, log group, stream, sequence)` of every row of the
    /// view stamped `stamp`, in the view's order.
    fn stamped_rows(
        store: &QueryStore,
        stamp: ViewStamp,
        length: usize,
    ) -> Vec<(i64, String, String, u64)> {
        let positions: Vec<usize> = (0..length).collect();
        store
            .copy_view_rows(stamp, &positions)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| match row {
                ResultRow::Event {
                    event,
                    log_group_name,
                } => Some((
                    event.timestamp,
                    log_group_name,
                    event.log_stream_name,
                    event.sequence,
                )),
                ResultRow::Aggregate { .. } => None,
            })
            .collect()
    }

    /// Evaluates `text` to its end over the view stamped `stamp`.
    fn evaluate_stamped(
        store: &QueryStore,
        stamp: ViewStamp,
        length: usize,
        text: &str,
    ) -> EvalOutput {
        let plan = parse_query(text).unwrap();
        let mut evaluation = Evaluation::new(&plan, length);
        while !evaluation.is_done() {
            assert!(store.evaluate_view_step(&plan, &mut evaluation, stamp));
        }
        evaluation.into_output().unwrap()
    }

    #[test]
    fn an_inflight_range_is_settled_once_and_leaves_the_registry() {
        // BR1.2, FR4.8: taken out of its holder, settled and unregistered
        // in one store lock.
        let store = QueryStore::new();
        let group = key("/g");
        let epoch = store.begin_run(vec![group.clone()]);
        let fetching = holder(vec![event(100, "a", 0), event(150, "a", 1)]);
        let id = store.register_inflight(&group, range(100, 199), Arc::clone(&fetching));
        assert_eq!(store.inflight_count(), 1);
        let partial = store.build_partial_view(epoch, 0, 1_000).unwrap();
        assert_eq!((partial.length, partial.inflight_count), (2, 2));
        assert_eq!(store.counts(&group), (0, 0), "not settled yet");

        assert_eq!(
            store.settle_inflight(id, true, LATER),
            Some(range(100, 199))
        );
        assert_eq!(store.inflight_count(), 0, "unregistered");
        assert!(
            fetching.lock().unwrap().is_empty(),
            "taken out of the holder"
        );
        assert_eq!(store.counts(&group), (2, 0), "settled once");
        assert_eq!(store.covered_ranges(&group), vec![range(100, 199)]);
        let after = store.build_partial_view(epoch, 0, 1_000).unwrap();
        assert_eq!(
            (after.length, after.inflight_count),
            (2, 0),
            "neither twice nor dropped"
        );
        assert_eq!(
            store.settle_inflight(id, true, LATER),
            None,
            "settled once only"
        );
        assert_eq!(store.counts(&group), (2, 0));
    }

    #[test]
    fn an_inflight_range_not_completed_settles_into_staging_and_covers_nothing() {
        // BR1.2, BR3.6, FR5.3: a failed or cancelled range is staging.
        let store = QueryStore::new();
        let group = key("/g");
        let epoch = store.begin_run(vec![group.clone()]);
        let id = store.register_inflight(
            &group,
            range(100, 199),
            holder(vec![event(120, "a", 0), event(300, "a", 1)]),
        );
        assert_eq!(store.settle_inflight(id, false, LATER), None);
        assert_eq!(
            store.counts(&group),
            (0, 1),
            "the event outside the range is dropped"
        );
        assert!(store.covered_ranges(&group).is_empty());
        assert_eq!(
            store.missing_ranges(&group, 100, 199),
            vec![range(100, 199)]
        );
        assert_eq!(
            store.build_view(epoch, 0, 1_000),
            Some(1),
            "used by this run"
        );
        store.begin_run(vec![group.clone()]);
        assert_eq!(store.counts(&group), (0, 0), "discarded by the next run");
    }

    #[test]
    fn partial_views_built_while_ranges_settle_never_count_an_event_twice_or_drop_one() {
        // FR4.8: there is no moment an event is in both its holder and the
        // settled holdings, nor in neither.
        let store = Arc::new(QueryStore::new());
        let groups: Vec<HoldingKey> = (0..4).map(|index| key(&format!("/g{index}"))).collect();
        let epoch = store.begin_run(groups.clone());
        let per_range = 500;
        let ids: Vec<u64> = groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                let base = i64::try_from(index).unwrap() * 1_000;
                let events = (0..per_range)
                    .map(|offset| event(base + offset, "s", 0))
                    .collect();
                store.register_inflight(group, range(base, base + 999), holder(events))
            })
            .collect();
        let total = groups.len() * usize::try_from(per_range).unwrap();
        let reader = {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                let mut seen = Vec::new();
                for _ in 0..200 {
                    let partial = store.build_partial_view(epoch, 0, 10_000).unwrap();
                    seen.push(partial.length);
                }
                seen
            })
        };
        for id in ids {
            store.settle_inflight(id, true, LATER);
        }
        let seen = reader.join().unwrap();
        assert!(seen.iter().all(|length| *length == total), "{seen:?}");
        assert_eq!(store.len(), total);
    }

    #[test]
    fn the_partial_view_orders_settled_and_inflight_events_as_a_result_does() {
        // BR1.3: (time, log group, stream, order in the stream).
        let store = QueryStore::new();
        let (orders, payments) = (key("/orders"), key("/payments"));
        let epoch = store.begin_run(vec![payments.clone(), orders.clone()]);
        store.settle_range(
            &payments,
            range(0, 999),
            vec![event(100, "s1", 0), event(200, "s1", 1)],
            true,
            LATER,
        );
        store.register_inflight(
            &orders,
            range(0, 999),
            holder(vec![
                event(100, "s2", 0),
                event(100, "s1", 0),
                event(150, "s1", 1),
            ]),
        );
        let partial = store.build_partial_view(epoch, 0, 999).unwrap();
        let stamp = partial.stamp;
        assert_eq!(
            stamped_rows(&store, stamp, partial.length),
            vec![
                (100, "/orders".to_string(), "s1".to_string(), 0),
                (100, "/orders".to_string(), "s2".to_string(), 0),
                (100, "/payments".to_string(), "s1".to_string(), 0),
                (150, "/orders".to_string(), "s1".to_string(), 1),
                (200, "/payments".to_string(), "s1".to_string(), 1),
            ]
        );
    }

    #[test]
    fn inflight_copies_take_the_numbers_settling_gives_and_leave_the_series_as_it_was() {
        // BR1.3: numbered on from the stream's series without moving it.
        let store = QueryStore::new();
        let group = key("/g");
        let epoch = store.begin_run(vec![group.clone()]);
        store.settle_range(
            &group,
            range(0, 99),
            vec![event(10, "a", 0), event(20, "a", 1)],
            true,
            LATER,
        );
        let id = store.register_inflight(
            &group,
            range(100, 199),
            holder(vec![
                event(110, "a", 0),
                event(120, "b", 0),
                event(130, "a", 1),
            ]),
        );
        for _ in 0..2 {
            let partial = store.build_partial_view(epoch, 0, 199).unwrap();
            let numbers: Vec<(String, u64)> = stamped_rows(&store, partial.stamp, partial.length)
                .into_iter()
                .map(|(_, _, stream, sequence)| (stream, sequence))
                .collect();
            assert_eq!(
                numbers,
                numbered(&[("a", 0), ("a", 1), ("a", 2), ("b", 0), ("a", 3)]),
                "the same numbers every time"
            );
        }
        store.settle_inflight(id, true, LATER);
        store.build_view(epoch, 0, 199);
        let settled: Vec<u64> = store
            .copy_rows(epoch, &[0, 1, 2, 3, 4])
            .unwrap()
            .iter()
            .filter_map(|row| row.event().map(|event| event.sequence))
            .collect();
        assert_eq!(settled, vec![0, 1, 2, 0, 3], "settling gave the same");
    }

    #[test]
    fn the_partial_view_leaves_out_what_is_outside_the_period_or_the_range() {
        // BR1.3 (FR review R-07), as build_view and settle_inflight do.
        let store = QueryStore::new();
        let group = key("/g");
        let epoch = store.begin_run(vec![group.clone()]);
        store.register_inflight(
            &group,
            range(100, 199),
            holder(vec![
                event(50, "a", 0),
                event(100, "a", 1),
                event(180, "a", 2),
                event(199, "a", 3),
                event(250, "a", 4),
            ]),
        );
        let partial = store.build_partial_view(epoch, 0, 190).unwrap();
        let times: Vec<i64> = stamped_rows(&store, partial.stamp, partial.length)
            .into_iter()
            .map(|(time, ..)| time)
            .collect();
        assert_eq!(times, vec![100, 180], "only the range and the period");
        assert_eq!(partial.inflight_count, 2);
        assert_eq!(
            store.build_partial_view(epoch + 1, 0, 190),
            None,
            "another epoch"
        );
    }

    #[test]
    fn cached_events_are_in_the_partial_view_as_settled_ones() {
        // BR1.3: what add_cached committed is part of the settled events.
        let store = QueryStore::new();
        let group = key("/g");
        let epoch = store.begin_run(vec![group.clone()]);
        store.add_cached(
            &group,
            &[range(0, 99)],
            vec![CachedEvent {
                timestamp: 50,
                ingestion_time: None,
                message: "cached".to_string(),
                log_stream_name: "a".to_string(),
                sequence: 9,
            }],
        );
        store.register_inflight(&group, range(100, 199), holder(vec![event(150, "a", 0)]));
        let partial = store.build_partial_view(epoch, 0, 199).unwrap();
        assert_eq!((partial.length, partial.inflight_count), (2, 1));
        assert_eq!(
            stamped_rows(&store, partial.stamp, partial.length),
            vec![
                (50, "/g".to_string(), "a".to_string(), 0),
                (150, "/g".to_string(), "a".to_string(), 1),
            ]
        );
    }

    #[test]
    fn the_view_version_moves_on_with_every_change_of_the_holdings() {
        // BR1.4: views, settling, the cache, a run start, a connection
        // change.
        let store = QueryStore::new();
        let group = key("/g");
        let mut versions = vec![store.view_version()];
        let epoch = store.begin_run(vec![group.clone()]);
        versions.push(store.view_version());
        store.build_view(epoch, 0, 999);
        versions.push(store.view_version());
        let id = store.register_inflight(&group, range(0, 99), holder(vec![event(5, "a", 0)]));
        let partial = store.build_partial_view(epoch, 0, 999).unwrap();
        assert_eq!(partial.stamp.version, store.view_version());
        versions.push(store.view_version());
        store.settle_inflight(id, true, LATER);
        versions.push(store.view_version());
        store.add_cached(&group, &[range(100, 199)], Vec::new());
        versions.push(store.view_version());
        store.settle_range(
            &group,
            range(200, 299),
            vec![event(250, "a", 0)],
            true,
            LATER,
        );
        versions.push(store.view_version());
        store.discard_all();
        versions.push(store.view_version());
        assert!(
            versions.windows(2).all(|pair| pair[0] < pair[1]),
            "{versions:?}"
        );
    }

    #[test]
    fn a_step_or_a_copy_over_a_view_that_changed_fails_and_does_nothing() {
        // BR1.4: the positions moved; the partial result is dropped.
        let store = QueryStore::new();
        let group = key("/g");
        let epoch = store.begin_run(vec![group.clone()]);
        let id = store.register_inflight(
            &group,
            range(0, 99),
            holder(vec![event(5, "a", 0), event(6, "a", 1)]),
        );
        let partial = store.build_partial_view(epoch, 0, 99).unwrap();
        let plan = parse_query("filter @message like 'a'").unwrap();
        let mut evaluation = Evaluation::new(&plan, partial.length);
        store.settle_inflight(id, true, LATER);
        assert!(!store.evaluate_view_step(&plan, &mut evaluation, partial.stamp));
        assert_eq!(evaluation.evaluated(), 0, "nothing was evaluated");
        assert_eq!(store.copy_view_rows(partial.stamp, &[0]), None);
        let again = store.build_partial_view(epoch, 0, 99).unwrap();
        assert!(store.evaluate_view_step(&plan, &mut evaluation, again.stamp));
        store.begin_run(vec![group]);
        assert_eq!(store.copy_view_rows(again.stamp, &[0]), None, "a new run");
    }

    #[test]
    fn a_partial_result_has_the_order_of_the_final_result_over_the_same_events() {
        // BR2.2 (FR4.2 as read): filter/fields, sort/limit and stats alike.
        let queries = [
            "filter @message like ':'",
            "fields @timestamp, @logStream | sort @logStream asc | limit 4",
            "stats count(*) as n by @logStream",
        ];
        for query in queries {
            let store = QueryStore::new();
            let (orders, payments) = (key("/orders"), key("/payments"));
            let epoch = store.begin_run(vec![orders.clone(), payments.clone()]);
            store.settle_range(
                &orders,
                range(0, 99),
                vec![event(10, "a", 0), event(50, "b", 0), event(50, "a", 1)],
                true,
                LATER,
            );
            let id = store.register_inflight(
                &payments,
                range(0, 99),
                holder(vec![
                    event(10, "a", 0),
                    event(50, "a", 1),
                    event(90, "c", 0),
                ]),
            );
            let partial = store.build_partial_view(epoch, 0, 99).unwrap();
            let early = evaluate_stamped(&store, partial.stamp, partial.length, query);
            let early_rows = match &early {
                EvalOutput::Events(output) => {
                    format!("{:?}", stamped_rows_at(&store, partial.stamp, &output.rows))
                }
                EvalOutput::Aggregates(output) => format!("{:?}", output.rows),
            };
            store.settle_inflight(id, true, LATER);
            let length = store.build_view(epoch, 0, 99).unwrap();
            let stamp = store.view_stamp(epoch).unwrap();
            let last = evaluate_stamped(&store, stamp, length, query);
            let last_rows = match &last {
                EvalOutput::Events(output) => {
                    format!("{:?}", stamped_rows_at(&store, stamp, &output.rows))
                }
                EvalOutput::Aggregates(output) => format!("{:?}", output.rows),
            };
            assert_eq!(early_rows, last_rows, "{query}");
        }
    }

    /// The `(timestamp, log group, stream, sequence)` of the rows at
    /// `positions` of the view stamped `stamp`.
    fn stamped_rows_at(
        store: &QueryStore,
        stamp: ViewStamp,
        positions: &[usize],
    ) -> Vec<(i64, String, String, u64)> {
        store
            .copy_view_rows(stamp, positions)
            .unwrap()
            .into_iter()
            .filter_map(|row| match row {
                ResultRow::Event {
                    event,
                    log_group_name,
                } => Some((
                    event.timestamp,
                    log_group_name,
                    event.log_stream_name,
                    event.sequence,
                )),
                ResultRow::Aggregate { .. } => None,
            })
            .collect()
    }

    // ---- U3 Step 22: the copy of a large holder in chunks (test first) --

    #[test]
    fn a_holder_copied_in_chunks_gives_the_events_and_order_of_one_copy() {
        // Equal times on a chunk's border stay in one chunk, so no event is
        // copied twice or left out and the order is the timeline's.
        let mut events = Vec::new();
        for time in [5, 5, 5, 6, 7, 7, 9, 9, 9, 9, 12] {
            let stream = if time % 2 == 0 { "a" } else { "b" };
            events.push(event(time, stream, 0));
        }
        events.push(event(200, "a", 0));
        let timeline = Mutex::new({
            let mut timeline = EventTimeline::new();
            timeline.append(events);
            timeline
        });
        let whole: Vec<(i64, String)> = timeline
            .lock()
            .unwrap()
            .events()
            .iter()
            .filter(|event| event.timestamp <= 100)
            .map(|event| (event.timestamp, event.log_stream_name.clone()))
            .collect();
        for chunk in [1, 2, 3, 4, 100] {
            let copied: Vec<(i64, String)> = copy_in_chunks(&timeline, range(0, 100), chunk)
                .into_iter()
                .map(|event| (event.timestamp, event.log_stream_name))
                .collect();
            assert_eq!(copied, whole, "chunk {chunk}");
        }
        assert!(copy_in_chunks(&timeline, range(300, 400), 2).is_empty());
    }

    #[test]
    fn a_holder_that_grows_between_chunks_gives_no_event_twice() {
        // A page added between two chunks may come before the place reached:
        // what is copied is still in order and each event at most once.
        let timeline = Mutex::new({
            let mut timeline = EventTimeline::new();
            timeline.append((0..10).map(|time| event(time, "a", 0)).collect());
            timeline
        });
        let mut copied = Vec::new();
        let mut cursor = None;
        let mut grew = false;
        loop {
            let (chunk, next) = copy_chunk(&timeline, range(0, 100), cursor, 3);
            copied.extend(chunk);
            if !grew {
                timeline
                    .lock()
                    .unwrap()
                    .append(vec![event(1, "b", 0), event(50, "b", 1)]);
                grew = true;
            }
            match next {
                Some(at) => cursor = Some(at),
                None => break,
            }
        }
        let times: Vec<i64> = copied.iter().map(|event| event.timestamp).collect();
        assert!(times.windows(2).all(|pair| pair[0] <= pair[1]), "{times:?}");
        let mut unique: Vec<(i64, String)> = copied
            .iter()
            .map(|event| (event.timestamp, event.log_stream_name.clone()))
            .collect();
        let count = unique.len();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), count, "no event twice");
        assert!(times.contains(&50), "a later page is copied when reached");
        assert_eq!(times.iter().filter(|time| **time < 10).count(), 10);
    }

    #[test]
    fn a_run_start_forgets_the_inflight_holders_of_the_run_before() {
        // BR1.2: the registry is the current run's.
        let store = QueryStore::new();
        let group = key("/g");
        store.begin_run(vec![group.clone()]);
        let id = store.register_inflight(&group, range(0, 99), holder(vec![event(5, "a", 0)]));
        let epoch = store.begin_run(vec![group.clone()]);
        assert_eq!(store.inflight_count(), 0);
        assert_eq!(store.settle_inflight(id, true, LATER), None);
        assert_eq!(store.counts(&group), (0, 0), "nothing of the old run");
        assert_eq!(
            store
                .build_partial_view(epoch, 0, 99)
                .map(|view| view.length),
            Some(0)
        );
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
        let Some(crate::query::evaluator::EvalOutput::Events(output)) = evaluation.into_output()
        else {
            panic!("{text:?} gave no events output");
        };
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
                    row.event()
                        .and_then(|event| {
                            serde_json::from_str::<serde_json::Value>(&event.message).ok()
                        })
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

    /// 261025-insights-stats U3 Step 22 (functional-spec 8): how long one
    /// range's holder is locked while a partial view copies it, and how
    /// long the whole partial view holds the store, with `count` events
    /// under fetch in one range (and as many settled). Run with `cargo test
    /// -p local-sights-core --release --lib query::store -- --ignored
    /// --nocapture`.
    fn measure_partial_view(count: usize) -> (Duration, Duration) {
        let store = QueryStore::new();
        let group = key(0);
        let epoch = store.begin_run(vec![group.clone()]);
        // `events` puts three events in each millisecond.
        let start_ms = 1_756_000_000_000;
        let wide = i64::try_from(count).unwrap_or(i64::MAX);
        let range = CoveredRange {
            start_ms,
            end_ms: start_ms + (wide - 1) / 3,
        };
        store.settle_range(&group, range, events(count, 0), true, i64::MAX);
        let mut timeline = EventTimeline::new();
        timeline.append(events(count, count));
        let holder = std::sync::Arc::new(Mutex::new(timeline));
        let later = CoveredRange {
            start_ms: range.end_ms + 1,
            end_ms: start_ms + wide,
        };
        store.register_inflight(&group, later, std::sync::Arc::clone(&holder));
        // The holder alone, one chunk a lock: the longest the fetch would wait
        // for its lock.
        let mut holder_lock = Duration::ZERO;
        let mut under_fetch = 0;
        let mut cursor = None;
        loop {
            let copying = Instant::now();
            let (part, next) = copy_chunk(&holder, later, cursor, INFLIGHT_COPY_CHUNK);
            holder_lock = holder_lock.max(copying.elapsed());
            under_fetch += part.len();
            match next {
                Some(at) => cursor = Some(at),
                None => break,
            }
        }
        let building = Instant::now();
        let partial = store
            .build_partial_view(epoch, range.start_ms, later.end_ms)
            .unwrap();
        let store_lock = building.elapsed();
        assert_eq!(
            (partial.length, partial.inflight_count),
            (count + under_fetch, under_fetch)
        );
        (holder_lock, store_lock)
    }

    #[test]
    #[ignore = "measures the copy of a large holder; run with --release --ignored"]
    fn the_copy_of_a_large_holder_under_fetch() {
        for count in [100_000, 300_000, 500_000] {
            let (holder_lock, store_lock) = measure_partial_view(count);
            println!(
                "{count} events under fetch (+{count} settled): holder locked at most {holder_lock:?} a chunk, partial view built (store locked) in {store_lock:?}"
            );
        }
    }
}
