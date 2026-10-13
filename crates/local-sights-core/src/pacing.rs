//! CallPacer: the per-region ceiling on `GetLogEvents` calls
//! (261012-parallel-fetch U1:BR1.1-BR1.5).
//!
//! One pacer is made per fetch, from the fetch's region, and shared by every
//! stream of that fetch. Before each `GetLogEvents` attempt (the first one
//! and every retry, U1:BR1.4, contract C1) the reader asks for a permit.
//! Permits are handed out one at a time, at least [`CallPacer::min_interval`]
//! apart, so no one-second window `[t, t + 1 s)` holds more permits than the
//! ceiling: half the region's AWS limit (U1:BR1.1, NFR3.2). There is no
//! burst allowance.
//!
//! The slot is reserved under a lock *before* waiting (U1:BR1.3, NFR3.3):
//! `slot = max(now, next_permit_at)` and `next_permit_at = slot + interval`
//! are decided while the lock is held, then the lock is released and the
//! reader sleeps until `slot`. Eight readers asking at the same moment
//! therefore get eight different slots. A stop signal ends the wait at once
//! and no call is made (U1:BR1.5); when the slot and the stop are ready
//! together the stop wins (`biased`, NFR4.1).
//!
//! tokio's timer wakes a reader on the first whole millisecond at or after
//! its slot, never before it. Rounding up never shortens a span, so the
//! one-second bound holds for the wake times too.
//!
//! What the pacer guarantees is the spacing of the *permits*. The actual
//! calls can drift from them: a reader that is woken late (for example while
//! the fetch task waits for the timeline lock) may call close to the next
//! reader, so one window could briefly hold a call more than the ceiling.
//! That is accepted: the ceiling is only half of the AWS limit, and that
//! margin absorbs the drift (NFR design review R-04).

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use tokio::time::Instant;

use crate::catalog::regions::STANDARD_REGIONS;
use crate::retry::StopSignal;

/// Calls per second for regions that are not in the table, and for a fetch
/// without a region (U1:BR1.2).
pub const DEFAULT_CEILING_PER_SECOND: u32 = 5;

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

/// Spaces the `GetLogEvents` attempts of one fetch (entities.md
/// PacingState). Shared by reference between the streams of the fetch.
#[derive(Debug)]
pub struct CallPacer {
    ceiling_per_second: u32,
    interval: Duration,
    /// The earliest time the next permit may be given.
    next_permit_at: Mutex<Instant>,
}

impl CallPacer {
    /// A pacer for the fetch's region; its first permit comes at once.
    pub fn for_region(region: Option<&str>) -> Self {
        let ceiling_per_second = RegionCallLimit::for_region(region).ceiling_per_second();
        Self {
            ceiling_per_second,
            interval: min_interval(ceiling_per_second),
            next_permit_at: Mutex::new(Instant::now()),
        }
    }

    /// Permits per second.
    pub fn ceiling_per_second(&self) -> u32 {
        self.ceiling_per_second
    }

    /// The time between two permits.
    pub fn min_interval(&self) -> Duration {
        self.interval
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

    /// Reserves the next slot under the lock and returns it (U1:BR1.3).
    /// The lock only guards one instant, which is never left half-written,
    /// so a poisoned lock is used as it is (NFR design review R-05).
    fn reserve_slot(&self) -> Instant {
        let mut next_permit_at = self
            .next_permit_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let slot = (*next_permit_at).max(Instant::now());
        *next_permit_at = slot + self.interval;
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
}
