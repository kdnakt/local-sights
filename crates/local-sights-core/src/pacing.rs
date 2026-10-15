//! CallPacer: the per-region ceiling on `GetLogEvents` calls
//! (261012-parallel-fetch U1:BR1.1-BR1.5), lowered on throttling and raised
//! again while no throttle comes (U2:BR2.1-BR2.6).
//!
//! One pacer is made per fetch, from the fetch's region, and shared by every
//! stream of that fetch. Before each `GetLogEvents` attempt (the first one
//! and every retry, U1:BR1.4, contract C1) the reader asks for a permit.
//! Permits are handed out one at a time, at least [`CallPacer::min_interval`]
//! apart, so no one-second window `[t, t + 1 s)` holds more permits than the
//! ceiling: half the region's AWS limit (U1:BR1.1, NFR3.2). There is no
//! burst allowance.
//!
//! Since U2 the spacing follows the *current rate*, which starts at the
//! ceiling (U2:BR2.6). Each `GetLogEvents` attempt that ends Throttled is
//! reported with [`CallPacer::report_throttled`] (U2:BR2.1, contract C1),
//! which halves the rate, rounded down and never below 1 (U2:BR2.2). After a
//! decrease, reports are not counted until one second has passed and every
//! slot reserved at the old rate is out (U2:BR2.3), so eight readers
//! throttled together, or calls reserved before the decrease, lower it only
//! once. Every five seconds without a report raise the rate by one, up to
//! the ceiling; the part of five seconds left over is carried to the next
//! raise (U2:BR2.4). The raise is applied when a slot is reserved and when a
//! throttle is reported, before anything else. A new rate spaces only the
//! slots reserved after it; slots already reserved keep their times
//! (U2:BR2.5). The rate never goes above the ceiling, so the one-second
//! bound above holds whatever the rate (NFR3.6).
//!
//! The slot is reserved under a lock *before* waiting (U1:BR1.3, NFR3.3):
//! `slot = max(now, next_permit_at)` and `next_permit_at = slot + interval`
//! are decided while the lock is held, then the lock is released and the
//! reader sleeps until `slot`. Eight readers asking at the same moment
//! therefore get eight different slots. A stop signal ends the wait at once
//! and no call is made (U1:BR1.5); when the slot and the stop are ready
//! together the stop wins (`biased`, NFR4.1), also at the lowest rate of 1,
//! where eight waiting readers would otherwise wait about eight seconds.
//!
//! tokio's timer wakes a reader on the first whole millisecond at or after
//! its slot, never before it (U1 code generation review R-01). Two
//! neighbouring wake times can therefore be up to 1 ms closer than the
//! interval (83 ms instead of 83.33 ms at 12 per second). The bound still
//! holds: every interval is at least the ceiling's, so two slots a ceiling's
//! worth of permits apart are at least one second apart, and rounding both
//! up to whole milliseconds keeps them at least one second apart (one
//! second is a whole number of milliseconds). So no window `[t, t + 1 s)`
//! holds more wake times than the ceiling, only the gap between two
//! neighbours can shrink by less than 1 ms.
//!
//! What the pacer guarantees is the spacing of the *permits*. The actual
//! calls can drift from them: a reader that is woken late (for example while
//! the fetch task waits for the timeline lock) may call close to the next
//! reader, so one window could briefly hold a call more than the ceiling.
//! That is accepted: the ceiling is only half of the AWS limit, and that
//! margin absorbs the drift (NFR design review R-04).

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::time::Instant;

use crate::catalog::regions::STANDARD_REGIONS;
use crate::retry::StopSignal;

/// Calls per second for regions that are not in the table, and for a fetch
/// without a region (U1:BR1.2).
pub const DEFAULT_CEILING_PER_SECOND: u32 = 5;

/// Time without a throttle report that raises the rate by one (U2:BR2.4).
pub const RECOVERY_STEP: Duration = Duration::from_secs(5);

/// The shortest time after a decrease during which reports do not lower
/// the rate again (U2:BR2.3).
pub const DECREASE_HOLD: Duration = Duration::from_secs(1);

/// The AWS default `GetLogEvents` limit of most public regions.
const COMMON_AWS_LIMIT_PER_SECOND: u32 = 25;

/// Regions whose AWS default `GetLogEvents` limit is not 25 per second
/// (per account and region, not adjustable). Every other region of the
/// standard partition's built-in list has 25.
const SPECIAL_AWS_LIMITS: &[(&str, u32)] = &[
    ("ap-northeast-3", 10),
    ("ap-southeast-3", 10),
    ("ca-west-1", 10),
    ("eu-central-1", 10),
    ("eu-west-1", 10),
    ("eu-west-3", 30),
    ("il-central-1", 10),
    ("us-west-2", 10),
];

/// The AWS limit of a region and the ceiling the app uses for it
/// (entities.md RegionCallLimit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionCallLimit {
    aws_limit_per_second: Option<u32>,
}

impl RegionCallLimit {
    /// The limit of `region`: from the table for a known public region
    /// (U1:BR1.1); unknown for any other region or no region (U1:BR1.2).
    pub fn for_region(region: Option<&str>) -> Self {
        let aws_limit_per_second = region.and_then(|code| {
            SPECIAL_AWS_LIMITS
                .iter()
                .find(|(special, _)| *special == code)
                .map(|(_, limit)| *limit)
                .or_else(|| {
                    STANDARD_REGIONS
                        .contains(&code)
                        .then_some(COMMON_AWS_LIMIT_PER_SECOND)
                })
        });
        Self {
            aws_limit_per_second,
        }
    }

    /// The AWS default limit, or 0 when the region is not in the table.
    pub fn aws_limit_per_second(self) -> u32 {
        self.aws_limit_per_second.unwrap_or(0)
    }

    /// Calls per second the app allows itself: half the AWS limit, rounded
    /// down (25 → 12, 10 → 5, 30 → 15), or 5 outside the table.
    pub fn ceiling_per_second(self) -> u32 {
        self.aws_limit_per_second
            .map_or(DEFAULT_CEILING_PER_SECOND, |limit| limit / 2)
    }
}

/// The shortest time between two permits for `ceiling` calls per second:
/// one second divided by the ceiling, rounded up to the nanosecond
/// (U1:BR1.3; 12 → 83,333,334 ns). A ceiling of 0 is treated as 1.
pub fn min_interval(ceiling: u32) -> Duration {
    const NANOS_PER_SECOND: u64 = 1_000_000_000;
    let ceiling = u64::from(ceiling.max(1));
    Duration::from_nanos(NANOS_PER_SECOND.div_ceil(ceiling))
}

/// Answer to a permit request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permit {
    /// Call `GetLogEvents` once now.
    Proceed,
    /// The fetch was told to stop; do not call (U1:BR1.5).
    Stopped,
}

/// The pacing of one fetch at one moment (U2 entities.md PacingState).
///
/// Every change is computed as a new value from the old one, with nothing
/// that can panic in between, and then written in one assignment under the
/// lock; see [`CallPacer`] for why that matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PacingState {
    /// Permits per second now, `1..=ceiling`.
    current_rate_per_second: u32,
    /// The earliest time the next permit may be given.
    next_permit_at: Instant,
    /// Reports before this instant do not lower the rate (U2:BR2.3);
    /// `None` until the first decrease.
    hold_decrease_until: Option<Instant>,
    /// Where the count of time without a throttle report starts (U2:BR2.4).
    quiet_since: Instant,
    /// The lowest rate of the fetch so far (U2:BR2.7).
    lowest_rate_per_second: u32,
}

impl PacingState {
    /// The state of a new fetch: at the ceiling, first permit at once
    /// (U2:BR2.6).
    fn starting(ceiling: u32, now: Instant) -> Self {
        Self {
            current_rate_per_second: ceiling,
            next_permit_at: now,
            hold_decrease_until: None,
            quiet_since: now,
            lowest_rate_per_second: ceiling,
        }
    }

    /// U2:BR2.4: one more permit per second for every whole five seconds
    /// since `quiet_since`, up to `ceiling`; the rest of the time is carried
    /// over. At the ceiling nothing changes.
    fn raised(self, ceiling: u32, now: Instant) -> Self {
        if self.current_rate_per_second >= ceiling {
            return self;
        }
        let quiet = now.saturating_duration_since(self.quiet_since);
        let steps = quiet.as_nanos() / RECOVERY_STEP.as_nanos();
        if steps == 0 {
            return self;
        }
        let steps = u32::try_from(steps).unwrap_or(u32::MAX);
        let carried_from = RECOVERY_STEP
            .checked_mul(steps)
            .and_then(|counted| self.quiet_since.checked_add(counted))
            .unwrap_or(now);
        Self {
            current_rate_per_second: self
                .current_rate_per_second
                .saturating_add(steps)
                .min(ceiling),
            quiet_since: carried_from,
            ..self
        }
    }

    /// U2:BR2.2, BR2.3: a throttle was reported at `now`. The raise due is
    /// applied first and the quiet count starts again; then, unless the
    /// last decrease still holds, the rate is halved (rounded down, at
    /// least 1) and held until one second has passed and the slots reserved
    /// so far are out.
    fn throttled(self, ceiling: u32, now: Instant) -> Self {
        let raised = Self {
            quiet_since: now,
            ..self.raised(ceiling, now)
        };
        if raised.hold_decrease_until.is_some_and(|until| now < until) {
            return raised;
        }
        let rate = (raised.current_rate_per_second / 2).max(1);
        Self {
            current_rate_per_second: rate,
            hold_decrease_until: Some((now + DECREASE_HOLD).max(raised.next_permit_at)),
            lowest_rate_per_second: raised.lowest_rate_per_second.min(rate),
            ..raised
        }
    }

    /// U1:BR1.3, U2:BR2.5: the raise due is applied, then the next slot is
    /// reserved with the interval of the rate now. Returns the new state
    /// and the slot.
    fn reserved(self, ceiling: u32, now: Instant) -> (Self, Instant) {
        let raised = self.raised(ceiling, now);
        let slot = raised.next_permit_at.max(now);
        let next = Self {
            next_permit_at: slot + min_interval(raised.current_rate_per_second),
            ..raised
        };
        (next, slot)
    }
}

/// Spaces the `GetLogEvents` attempts of one fetch (entities.md
/// PacingState). Shared by reference between the streams of the fetch.
///
/// The whole [`PacingState`] sits behind one lock: lowering, raising and
/// reserving a slot are each decided in one hold of it. Since U2 the lock
/// guards several fields, not one instant, so a poisoned lock is safe to
/// use only because every change is computed first and written in one
/// assignment: a panic while the lock is held (there is nothing that panics
/// between computing and writing) leaves the value from before or after one
/// whole operation, never a mix. A poisoned lock is therefore used as it is
/// (U1 NFR design review R-05, U2 entities.md PacingState).
#[derive(Debug)]
pub struct CallPacer {
    ceiling_per_second: u32,
    state: Mutex<PacingState>,
}

impl CallPacer {
    /// A pacer for the fetch's region, at the region's ceiling; its first
    /// permit comes at once.
    pub fn for_region(region: Option<&str>) -> Self {
        let ceiling_per_second = RegionCallLimit::for_region(region)
            .ceiling_per_second()
            .max(1);
        Self {
            ceiling_per_second,
            state: Mutex::new(PacingState::starting(ceiling_per_second, Instant::now())),
        }
    }

    /// The highest permits per second, the region's ceiling; it does not
    /// change during the fetch.
    pub fn ceiling_per_second(&self) -> u32 {
        self.ceiling_per_second
    }

    /// Permits per second now (U2:BR2.5), as of the last report or slot.
    pub fn current_rate_per_second(&self) -> u32 {
        self.lock().current_rate_per_second
    }

    /// The lowest permits per second of the fetch so far; the ceiling when
    /// the rate was never lowered (U2:BR2.7).
    pub fn lowest_rate_per_second(&self) -> u32 {
        self.lock().lowest_rate_per_second
    }

    /// The time between two permits at the current rate: one second
    /// divided by it, rounded up to the nanosecond (U1:BR1.3, U2:BR2.5).
    pub fn min_interval(&self) -> Duration {
        min_interval(self.current_rate_per_second())
    }

    /// A `GetLogEvents` attempt ended Throttled (U2:BR2.1, contract C1):
    /// halves the rate unless the last decrease still holds, and starts the
    /// count of quiet time again (U2:BR2.2-BR2.4). Slots already reserved
    /// keep their times.
    pub fn report_throttled(&self) {
        let mut state = self.lock();
        *state = state.throttled(self.ceiling_per_second, Instant::now());
    }

    fn lock(&self) -> MutexGuard<'_, PacingState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits for the next permit, unless `stop` fires first. A stopped
    /// signal gives [`Permit::Stopped`] at once. The reserved slot is not
    /// given back when the wait is stopped, which only spaces later permits
    /// more widely.
    pub async fn permit(&self, stop: &StopSignal) -> Permit {
        if stop.is_stopped() {
            return Permit::Stopped;
        }
        let slot = self.reserve_slot();
        tokio::select! {
            biased;
            () = stop.stopped() => Permit::Stopped,
            () = tokio::time::sleep_until(slot) => Permit::Proceed,
        }
    }

    /// Reserves the next slot under the lock and returns it, after applying
    /// the raise due (U1:BR1.3, U2:BR2.4, BR2.5).
    fn reserve_slot(&self) -> Instant {
        let mut state = self.lock();
        let (next, slot) = state.reserved(self.ceiling_per_second, Instant::now());
        *state = next;
        slot
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use tokio::time::Instant;

    use super::*;
    use crate::retry::{StopReason, stop_pair};

    #[test]
    fn the_ceiling_is_half_the_region_limit_rounded_down() {
        // 25 per second in most public regions (BR1.1).
        for region in ["ap-northeast-1", "us-east-1", "ap-southeast-6"] {
            assert_eq!(
                RegionCallLimit::for_region(Some(region)).ceiling_per_second(),
                12
            );
            assert_eq!(
                RegionCallLimit::for_region(Some(region)).aws_limit_per_second(),
                25
            );
        }
        // 10 per second in seven regions.
        for region in [
            "us-west-2",
            "ap-northeast-3",
            "ap-southeast-3",
            "ca-west-1",
            "eu-central-1",
            "eu-west-1",
            "il-central-1",
        ] {
            let limit = RegionCallLimit::for_region(Some(region));
            assert_eq!(limit.aws_limit_per_second(), 10, "{region}");
            assert_eq!(limit.ceiling_per_second(), 5, "{region}");
        }
        // 30 per second in eu-west-3.
        let paris = RegionCallLimit::for_region(Some("eu-west-3"));
        assert_eq!(paris.aws_limit_per_second(), 30);
        assert_eq!(paris.ceiling_per_second(), 15);
    }

    #[test]
    fn an_unknown_or_missing_region_gets_five() {
        for region in [
            Some("zz-local-1"),
            Some("us-gov-west-1"),
            Some("cn-north-1"),
            None,
        ] {
            assert_eq!(
                RegionCallLimit::for_region(region).ceiling_per_second(),
                5,
                "{region:?}"
            );
        }
        assert_eq!(CallPacer::for_region(None).ceiling_per_second(), 5);
        assert_eq!(
            CallPacer::for_region(Some("ap-northeast-1")).ceiling_per_second(),
            12
        );
    }

    #[test]
    fn the_interval_is_rounded_up_to_the_nanosecond() {
        assert_eq!(min_interval(12), Duration::from_nanos(83_333_334));
        assert_eq!(min_interval(5), Duration::from_millis(200));
        assert_eq!(min_interval(15), Duration::from_nanos(66_666_667));
        assert_eq!(min_interval(0), Duration::from_secs(1), "never zero calls");
        assert_eq!(
            CallPacer::for_region(Some("ap-northeast-1")).min_interval(),
            Duration::from_nanos(83_333_334)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_first_permit_comes_at_once() {
        let pacer = CallPacer::for_region(Some("ap-northeast-1"));
        let signal = crate::retry::StopSignal::never();
        let started = Instant::now();
        assert_eq!(pacer.permit(&signal).await, Permit::Proceed);
        assert_eq!(started.elapsed(), Duration::ZERO);
        assert_eq!(pacer.permit(&signal).await, Permit::Proceed);
        // The next one waits for its slot; tokio's timer wakes on the next
        // whole millisecond after it, never before.
        let waited = started.elapsed();
        assert!(waited >= min_interval(12), "{waited:?}");
        assert!(
            waited < min_interval(12) + Duration::from_millis(1),
            "{waited:?}"
        );
    }

    /// Eight readers ask at the same moment, then again after a retry wait.
    async fn permit_times(pacer: &CallPacer, readers: usize, rounds: usize) -> Vec<Instant> {
        let signal = crate::retry::StopSignal::never();
        let times = Mutex::new(Vec::new());
        let reader = || async {
            for _ in 0..rounds {
                assert_eq!(pacer.permit(&signal).await, Permit::Proceed);
                times.lock().unwrap().push(Instant::now());
            }
        };
        futures_util::future::join_all((0..readers).map(|_| reader())).await;
        let mut times = times.into_inner().unwrap();
        times.sort();
        times
    }

    #[tokio::test(start_paused = true)]
    async fn eight_readers_asking_at_once_get_spaced_permits() {
        // The reserved slots of eight readers asking at the same moment are
        // all one interval apart (BR1.3), also for Tokyo's 83,333,334 ns.
        let tokyo = CallPacer::for_region(Some("ap-northeast-1"));
        let slots = Mutex::new(Vec::new());
        let ask = || async {
            for _ in 0..3 {
                slots.lock().unwrap().push(tokyo.reserve_slot());
                tokio::task::yield_now().await;
            }
        };
        futures_util::future::join_all((0..8).map(|_| ask())).await;
        let mut slots = slots.into_inner().unwrap();
        slots.sort();
        slots.dedup();
        assert_eq!(slots.len(), 24, "no two readers share a slot");
        for pair in slots.windows(2) {
            assert_eq!(pair[1] - pair[0], min_interval(12));
        }

        // The permits themselves come no earlier than their slots: with a
        // whole-millisecond interval the wake times are exactly 200 ms apart.
        let pacer = CallPacer::for_region(Some("eu-west-1"));
        let times = permit_times(&pacer, 8, 3).await;
        assert_eq!(times.len(), 24);
        for pair in times.windows(2) {
            assert_eq!(pair[1] - pair[0], min_interval(5));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn no_one_second_window_holds_more_than_the_ceiling() {
        for (region, ceiling) in [
            (Some("ap-northeast-1"), 12),
            (Some("eu-west-1"), 5),
            (None, 5),
        ] {
            let pacer = CallPacer::for_region(region);
            let times = permit_times(&pacer, 8, 5).await;
            for (index, start) in times.iter().enumerate() {
                let in_window = times[index..]
                    .iter()
                    .take_while(|time| **time - *start < Duration::from_secs(1))
                    .count();
                assert!(in_window <= ceiling, "{region:?}: {in_window} > {ceiling}");
            }
            // The ceiling is reached, not only respected.
            let first_second = times
                .iter()
                .take_while(|time| **time - times[0] < Duration::from_secs(1))
                .count();
            assert_eq!(first_second, ceiling, "{region:?}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_stop_while_waiting_for_a_permit_ends_the_wait_without_a_permit() {
        let pacer = CallPacer::for_region(None);
        let (handle, signal) = stop_pair();
        assert_eq!(pacer.permit(&signal).await, Permit::Proceed);
        let started = Instant::now();
        let stop_later = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            handle.stop(StopReason::Close);
        };
        let (permit, ()) = tokio::join!(pacer.permit(&signal), stop_later);
        assert_eq!(permit, Permit::Stopped);
        assert_eq!(
            started.elapsed(),
            Duration::from_millis(50),
            "ends at the stop, not at the 200 ms slot"
        );
        assert_eq!(
            pacer.permit(&signal).await,
            Permit::Stopped,
            "a stopped signal gives no more permits"
        );
        assert_eq!(started.elapsed(), Duration::from_millis(50));
    }

    #[tokio::test(start_paused = true)]
    async fn when_the_permit_and_the_stop_are_ready_together_the_stop_wins() {
        use futures_util::FutureExt;

        let pacer = CallPacer::for_region(None);
        let (handle, signal) = stop_pair();
        assert_eq!(pacer.permit(&signal).await, Permit::Proceed);
        // The second permit reserves the 200 ms slot and waits.
        let waiting = pacer.permit(&signal);
        tokio::pin!(waiting);
        assert_eq!((&mut waiting).now_or_never(), None);
        // The slot passes and the stop arrives before the waiter runs again:
        // at its next poll both are ready, and the stop is taken.
        tokio::time::advance(min_interval(5)).await;
        handle.stop(StopReason::Close);
        assert_eq!((&mut waiting).now_or_never(), Some(Permit::Stopped));
    }

    // ---- 261012-parallel-fetch U2: lowering and raising the rate ----

    const SECOND: Duration = Duration::from_secs(1);

    fn millis(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    /// Moves the paused clock forward to `at`.
    async fn advance_to(at: Instant) {
        tokio::time::advance(at - Instant::now()).await;
    }

    /// Before this instant a report does not lower the rate again.
    fn hold_decrease_until(pacer: &CallPacer) -> Option<Instant> {
        pacer.state.lock().unwrap().hold_decrease_until
    }

    /// A Tokyo pacer lowered 12 → 6 → 3 by two reports one second apart,
    /// returned at the moment of the second report.
    async fn tokyo_lowered_to_three() -> CallPacer {
        let pacer = CallPacer::for_region(Some("ap-northeast-1"));
        pacer.report_throttled();
        tokio::time::advance(SECOND).await;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 3);
        pacer
    }

    #[tokio::test(start_paused = true)]
    async fn a_report_halves_the_rate_down_to_one() {
        // U2:BR2.2: 12 → 6 → 3 → 1 → 1, each report after the last hold.
        let pacer = CallPacer::for_region(Some("ap-northeast-1"));
        assert_eq!(pacer.current_rate_per_second(), 12);
        let mut rates = Vec::new();
        for _ in 0..4 {
            pacer.report_throttled();
            let rate = pacer.current_rate_per_second();
            rates.push(rate);
            assert_eq!(pacer.min_interval(), min_interval(rate), "rate {rate}");
            tokio::time::advance(SECOND).await;
        }
        assert_eq!(rates, [6, 3, 1, 1]);
        assert_eq!(pacer.lowest_rate_per_second(), 1);
        assert_eq!(pacer.ceiling_per_second(), 12, "the ceiling never moves");

        // A ceiling of 5 (no region): 5 → 2 → 1, rounded down.
        let pacer = CallPacer::for_region(None);
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 2);
        tokio::time::advance(SECOND).await;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn reports_that_come_together_lower_the_rate_once() {
        // U2:BR2.3: one decrease, then seven reports inside its second.
        let pacer = CallPacer::for_region(Some("ap-northeast-1"));
        pacer.report_throttled();
        for _ in 0..7 {
            tokio::time::advance(millis(140)).await;
            pacer.report_throttled();
        }
        assert_eq!(pacer.current_rate_per_second(), 6);
        assert_eq!(pacer.lowest_rate_per_second(), 6);

        // Eight readers throttled at the same moment.
        let together = CallPacer::for_region(Some("ap-northeast-1"));
        for _ in 0..8 {
            together.report_throttled();
        }
        assert_eq!(together.current_rate_per_second(), 6);

        // One second after the decrease the next report counts again.
        tokio::time::advance(millis(20)).await;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn reports_for_slots_reserved_before_a_decrease_do_not_lower_again() {
        // Functional design review R-07: at rate 3 with slots reserved
        // 2.7 s ahead (eight readers), a report lowers the rate to 1 and
        // holds until those slots are out, not for one second only.
        let pacer = tokyo_lowered_to_three().await;
        tokio::time::advance(SECOND).await;
        let lowered_at = Instant::now();
        for _ in 0..8 {
            pacer.reserve_slot();
        }
        let reserved_until = lowered_at + min_interval(3) * 8;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 1);
        assert_eq!(hold_decrease_until(&pacer), Some(reserved_until));
        for at in [1_500, 2_500] {
            advance_to(lowered_at + millis(at)).await;
            pacer.report_throttled();
            assert_eq!(
                hold_decrease_until(&pacer),
                Some(reserved_until),
                "the report at {at} ms is not counted"
            );
        }
        advance_to(lowered_at + Duration::from_secs(3)).await;
        pacer.report_throttled();
        assert_eq!(
            hold_decrease_until(&pacer),
            Some(Instant::now() + SECOND),
            "counted once the reserved slots are out"
        );
        assert_eq!(pacer.current_rate_per_second(), 1);

        // The same hold seen in the rate: lowered to 3 with slots reserved
        // 2.7 s ahead at rate 6, reports at 1.5 s and 2.5 s keep 3 and a
        // report at 3.0 s lowers it to 1 (functional-spec 4).
        let pacer = CallPacer::for_region(Some("ap-northeast-1"));
        pacer.report_throttled();
        tokio::time::advance(SECOND).await;
        let lowered_at = Instant::now();
        for _ in 0..16 {
            pacer.reserve_slot();
        }
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 3);
        assert_eq!(
            hold_decrease_until(&pacer),
            Some(lowered_at + min_interval(6) * 16)
        );
        for at in [1_500, 2_500] {
            advance_to(lowered_at + millis(at)).await;
            pacer.report_throttled();
            assert_eq!(pacer.current_rate_per_second(), 3, "at {at} ms");
        }
        advance_to(lowered_at + Duration::from_secs(3)).await;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn quiet_time_raises_the_rate_by_one_every_five_seconds_up_to_the_ceiling() {
        // U2:BR2.4: 3 → 4 → 5 → 12 → 12 at 5, 10, 45 and 60 s.
        let pacer = tokyo_lowered_to_three().await;
        let lowered_at = Instant::now();
        advance_to(lowered_at + millis(4_999)).await;
        pacer.reserve_slot();
        assert_eq!(pacer.current_rate_per_second(), 3, "not before 5 s");
        let mut rates = Vec::new();
        for at in [5, 10, 45, 60] {
            advance_to(lowered_at + Duration::from_secs(at)).await;
            pacer.reserve_slot();
            rates.push(pacer.current_rate_per_second());
        }
        assert_eq!(rates, [4, 5, 12, 12]);
        assert_eq!(pacer.min_interval(), min_interval(12));
        assert_eq!(pacer.lowest_rate_per_second(), 3, "the lowest is kept");
    }

    #[tokio::test(start_paused = true)]
    async fn a_part_of_five_seconds_is_carried_over() {
        // U2:BR2.4: +1 at 7 s counts from 5 s, so 10 s gives +1 more.
        let pacer = tokyo_lowered_to_three().await;
        let lowered_at = Instant::now();
        advance_to(lowered_at + Duration::from_secs(7)).await;
        pacer.reserve_slot();
        assert_eq!(pacer.current_rate_per_second(), 4);
        advance_to(lowered_at + Duration::from_secs(10)).await;
        pacer.reserve_slot();
        assert_eq!(pacer.current_rate_per_second(), 5);
    }

    #[tokio::test(start_paused = true)]
    async fn every_report_starts_the_quiet_count_again() {
        // U2:BR2.4: a report 4 s after the decrease to 3 lowers to 1, and
        // the next +1 comes 5 s after that report.
        let pacer = tokyo_lowered_to_three().await;
        let lowered_at = Instant::now();
        advance_to(lowered_at + Duration::from_secs(4)).await;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 1);
        advance_to(lowered_at + Duration::from_secs(8)).await;
        pacer.reserve_slot();
        assert_eq!(pacer.current_rate_per_second(), 1, "counted from 4 s");
        advance_to(lowered_at + Duration::from_secs(9)).await;
        pacer.reserve_slot();
        assert_eq!(pacer.current_rate_per_second(), 2);

        // A report inside the hold lowers nothing but restarts the count.
        let pacer = tokyo_lowered_to_three().await;
        let lowered_at = Instant::now();
        advance_to(lowered_at + millis(500)).await;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 3);
        advance_to(lowered_at + Duration::from_secs(5)).await;
        pacer.reserve_slot();
        assert_eq!(pacer.current_rate_per_second(), 3, "counted from 0.5 s");
        advance_to(lowered_at + millis(5_500)).await;
        pacer.reserve_slot();
        assert_eq!(pacer.current_rate_per_second(), 4);
    }

    #[tokio::test(start_paused = true)]
    async fn quiet_time_before_a_report_is_applied_before_halving() {
        // U2:BR2.2, BR2.4: 12 s after the decrease to 3 with no report and
        // no slot, the report first raises 3 → 5, then halves it to 2.
        let pacer = tokyo_lowered_to_three().await;
        let lowered_at = Instant::now();
        advance_to(lowered_at + Duration::from_secs(12)).await;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 2, "not 3 / 2 = 1");
        assert_eq!(pacer.lowest_rate_per_second(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_new_rate_spaces_only_the_slots_reserved_after_it() {
        // U2:BR2.5: slots reserved at 12 keep their times; the slots after
        // the decrease are spaced by the interval of 6.
        let pacer = CallPacer::for_region(Some("ap-northeast-1"));
        let before: Vec<Instant> = (0..3).map(|_| pacer.reserve_slot()).collect();
        pacer.report_throttled();
        assert_eq!(pacer.min_interval(), min_interval(6));
        let after: Vec<Instant> = (0..3).map(|_| pacer.reserve_slot()).collect();
        for pair in before.windows(2) {
            assert_eq!(pair[1] - pair[0], min_interval(12));
        }
        assert_eq!(
            after[0] - before[2],
            min_interval(12),
            "the slot after the last one reserved at 12 is where it was due"
        );
        for pair in after.windows(2) {
            assert_eq!(pair[1] - pair[0], min_interval(6));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn every_fetch_starts_at_the_ceiling() {
        // U2:BR2.6: a pacer is made per fetch, so the next fetch starts at
        // the region's ceiling whatever the last one came down to.
        let lowered = tokyo_lowered_to_three().await;
        tokio::time::advance(SECOND).await;
        lowered.report_throttled();
        assert_eq!(lowered.current_rate_per_second(), 1);
        let next = CallPacer::for_region(Some("ap-northeast-1"));
        assert_eq!(next.current_rate_per_second(), 12);
        assert_eq!(next.lowest_rate_per_second(), 12);
        assert_eq!(next.min_interval(), min_interval(12));
        assert_eq!(hold_decrease_until(&next), None);
        let unknown = CallPacer::for_region(None);
        assert_eq!(unknown.current_rate_per_second(), 5);
        assert_eq!(unknown.lowest_rate_per_second(), 5);
    }

    #[tokio::test(start_paused = true)]
    async fn the_lowest_rate_stays_once_the_rate_comes_back() {
        // U2:BR2.7: the lowest rate of the fetch, for the summary line.
        let pacer = CallPacer::for_region(Some("eu-west-1"));
        assert_eq!(pacer.lowest_rate_per_second(), 5, "the ceiling at first");
        pacer.report_throttled();
        tokio::time::advance(SECOND).await;
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 1);
        tokio::time::advance(Duration::from_secs(60)).await;
        pacer.reserve_slot();
        assert_eq!(pacer.current_rate_per_second(), 5, "back at the ceiling");
        assert_eq!(pacer.lowest_rate_per_second(), 1);
        pacer.report_throttled();
        assert_eq!(pacer.current_rate_per_second(), 2);
        assert_eq!(pacer.lowest_rate_per_second(), 1, "never raised");
    }
}
