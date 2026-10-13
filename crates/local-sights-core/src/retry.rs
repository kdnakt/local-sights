//! RetryPolicy: which failures are retried, how long to wait and when one
//! request gives up (U3:BR2.1-BR2.3), plus the stop signal used by every
//! wait.
//!
//! The decisions are pure functions; [`Retrier`] runs them around one API
//! call, waiting with `tokio::time::sleep` and stopping as soon as the
//! [`StopSignal`] fires. The random spread of each wait comes from a
//! [`JitterSource`], so tests can fix it.
//!
//! Since 261012-parallel-fetch U3 (ADR-007, BR3.6) this module only covers
//! one request and the stop signal: counting the streams that used up their
//! retries in a row, and giving up on the streams not started yet, belong to
//! the [`StreamSchedule`](crate::schedule::StreamSchedule).
//!
//! Since 261012-parallel-fetch U1 the stop signal carries a reason
//! ([`StopReason`], contract C7): Close always wins over Cancel (BR1.20).
//! It replaced the reasonless abort signal of U3.

use std::future::Future;
use std::time::Duration;

use tokio::sync::watch;

use crate::failure::{ApiFailure, FailureKind};
use crate::pacing::Permit;

/// Rules of retrying (entities.md RetryPolicy, Q1).
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// Wait before the n-th retry, in milliseconds (1, 2, 4, 8, 16 s).
    pub base_delays_ms: Vec<u64>,
    /// Each wait is spread by up to this ratio either way (0.2 = ±20%).
    pub jitter_ratio: f64,
    /// Retries per request; the first call plus this many at most.
    pub max_retries: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_delays_ms: vec![1_000, 2_000, 4_000, 8_000, 16_000],
            jitter_ratio: 0.2,
            max_retries: 5,
        }
    }
}

/// What to do after a failed call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// Wait this long, then send the same request again.
    RetryAfter(Duration),
    /// Stop; `exhausted` is true when a retryable failure used up every
    /// retry, false when the failure is not retryable at all.
    GiveUp {
        /// Whether the retries were used up.
        exhausted: bool,
    },
}

impl RetryPolicy {
    /// Only throttling and network failures are retried (BR2.1).
    pub fn is_retryable(&self, kind: FailureKind) -> bool {
        kind.is_retryable()
    }

    /// Wait before the `retry_number`-th retry (from 1): the base delay
    /// times `1 + jitter`, with `jitter` clamped to ±`jitter_ratio`.
    /// `None` when that retry is beyond `max_retries`.
    pub fn delay_before_retry(&self, retry_number: u32, jitter: f64) -> Option<Duration> {
        if retry_number == 0 || retry_number > self.max_retries {
            return None;
        }
        let index = usize::try_from(retry_number - 1).ok()?;
        let base = *self
            .base_delays_ms
            .get(index)
            .or(self.base_delays_ms.last())?;
        let ratio = self.jitter_ratio.abs();
        let spread = if jitter.is_finite() {
            jitter.clamp(-ratio, ratio)
        } else {
            0.0
        };
        let millis = (base as f64 * (1.0 + spread)).round().max(0.0);
        Some(Duration::from_millis(millis as u64))
    }

    /// Decides after a failure of `kind`, given how many retries this
    /// request has already had (BR2.1, BR2.2).
    pub fn decide(&self, kind: FailureKind, retries_done: u32, jitter: f64) -> RetryDecision {
        if !self.is_retryable(kind) {
            return RetryDecision::GiveUp { exhausted: false };
        }
        match self.delay_before_retry(retries_done.saturating_add(1), jitter) {
            Some(delay) => RetryDecision::RetryAfter(delay),
            None => RetryDecision::GiveUp { exhausted: true },
        }
    }
}

/// Counts retries per request and in total for one stream (BR2.2), and
/// since 261012-parallel-fetch U1 the attempts and the throttled attempts
/// over all its requests (NFR2.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetryCounter {
    current_request: u32,
    total: u32,
    attempts: u32,
    throttled: u32,
}

impl RetryCounter {
    /// A counter with no retries.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retries of the request in progress.
    pub fn current_request(&self) -> u32 {
        self.current_request
    }

    /// Retries of every request so far (`StreamFetchOutcome.retryCount`).
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Records one retry of the current request.
    pub fn record_retry(&mut self) {
        self.current_request = self.current_request.saturating_add(1);
        self.total = self.total.saturating_add(1);
    }

    /// The request succeeded: the next request counts from 0 again.
    pub fn record_success(&mut self) {
        self.current_request = 0;
    }

    /// Attempts made, over every request so far (each one an API call).
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Attempts that failed as Throttled, over every request so far.
    pub fn throttled(&self) -> u32 {
        self.throttled
    }

    /// Records one attempt and, when it failed, the kind of its failure.
    pub fn record_attempt(&mut self, failure: Option<FailureKind>) {
        self.attempts = self.attempts.saturating_add(1);
        if failure == Some(FailureKind::Throttled) {
            self.throttled = self.throttled.saturating_add(1);
        }
    }
}

/// Source of the random spread of each wait.
pub trait JitterSource: Send + Sync {
    /// A value in `[-ratio, ratio]`.
    fn sample(&self, ratio: f64) -> f64;
}

/// Uniform random spread (production).
#[derive(Debug, Clone, Copy, Default)]
pub struct RandomJitter;

impl JitterSource for RandomJitter {
    fn sample(&self, ratio: f64) -> f64 {
        (fastrand::f64() * 2.0 - 1.0) * ratio
    }
}

/// Always the same spread, clamped to the ratio (tests).
#[derive(Debug, Clone, Copy, Default)]
pub struct FixedJitter(pub f64);

impl JitterSource for FixedJitter {
    fn sample(&self, ratio: f64) -> f64 {
        let ratio = ratio.abs();
        self.0.clamp(-ratio, ratio)
    }
}

/// Why a fetch is told to stop (261012-parallel-fetch contract C7).
///
/// U1 only sends [`StopReason::Close`] (the window closes); the [Cancel]
/// button that sends [`StopReason::Cancel`] comes with U5. The strength
/// rule is already in force: Close always wins over Cancel (U1:BR1.20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The person cancelled the fetch (U5).
    Cancel,
    /// The window is closing: everything fetched is discarded (U1:BR1.16).
    Close,
}

impl StopReason {
    /// The reason after `incoming` arrives while `current` holds
    /// (U1:BR1.20): the first reason sent, except that Close beats Cancel
    /// and nothing beats Close. Sending the same reason again changes
    /// nothing.
    pub fn stronger(current: Option<StopReason>, incoming: StopReason) -> StopReason {
        match (current, incoming) {
            (Some(StopReason::Close), _) | (_, StopReason::Close) => StopReason::Close,
            (Some(StopReason::Cancel) | None, StopReason::Cancel) => StopReason::Cancel,
        }
    }
}

/// The caller's side of a stop signal.
#[derive(Debug)]
pub struct StopHandle {
    sender: watch::Sender<Option<StopReason>>,
}

/// The fetch's side of a stop signal (U3:BR2.3, BR5.5; 261012-parallel-fetch
/// contract C7). Every wait of a fetch (a retry wait, a call permit) ends as
/// soon as it fires.
#[derive(Debug, Clone)]
pub struct StopSignal {
    receiver: watch::Receiver<Option<StopReason>>,
}

/// A connected handle and signal, not stopped yet.
pub fn stop_pair() -> (StopHandle, StopSignal) {
    let (sender, receiver) = watch::channel(None);
    (StopHandle { sender }, StopSignal { receiver })
}

impl StopHandle {
    /// Tells the fetch to stop for `reason`, keeping the stronger reason
    /// when it was already stopped (U1:BR1.20). Returns whether the reason
    /// changed; the waiting side is woken only then.
    pub fn stop(&self, reason: StopReason) -> bool {
        self.sender.send_if_modified(|current| {
            let next = StopReason::stronger(*current, reason);
            let changed = *current != Some(next);
            *current = Some(next);
            changed
        })
    }

    /// A new signal connected to this handle.
    pub fn signal(&self) -> StopSignal {
        StopSignal {
            receiver: self.sender.subscribe(),
        }
    }
}

impl StopSignal {
    /// A signal that never fires (the `fetch_check` example).
    pub fn never() -> Self {
        let (handle, signal) = stop_pair();
        // Dropping the handle closes the channel; `stopped` then never
        // completes and `is_stopped` stays false.
        drop(handle);
        signal
    }

    /// The reason the fetch was told to stop, if it was.
    pub fn reason(&self) -> Option<StopReason> {
        *self.receiver.borrow()
    }

    /// Whether the fetch was told to stop.
    pub fn is_stopped(&self) -> bool {
        self.reason().is_some()
    }

    /// Completes when the fetch is told to stop; never, when the handle was
    /// dropped without stopping.
    pub async fn stopped(&self) {
        let mut receiver = self.receiver.clone();
        if receiver.wait_for(Option::is_some).await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// How a wait ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitOutcome {
    /// The whole delay passed.
    Elapsed,
    /// The stop signal fired first (BR2.3).
    Stopped,
}

/// Waits `delay` unless the stop signal fires first. When both are ready
/// at once, the stop wins (`biased`, 261012-parallel-fetch NFR4.1).
pub async fn wait_or_stop(delay: Duration, signal: &StopSignal) -> WaitOutcome {
    if signal.is_stopped() {
        return WaitOutcome::Stopped;
    }
    tokio::select! {
        biased;
        () = signal.stopped() => WaitOutcome::Stopped,
        () = tokio::time::sleep(delay) => WaitOutcome::Elapsed,
    }
}

/// Result of one request with its retries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallResult<T> {
    /// The request succeeded.
    Succeeded(T),
    /// The request failed for good.
    Failed {
        /// The last failure (safe detail only).
        failure: ApiFailure,
        /// Whether a retryable failure used up every retry.
        exhausted: bool,
    },
    /// The stop signal fired; the request was not sent again.
    Aborted,
}

/// Runs one request with the retry rules and the stop signal.
#[derive(Clone, Copy)]
pub struct Retrier<'a> {
    policy: &'a RetryPolicy,
    jitter: &'a dyn JitterSource,
    stop: &'a StopSignal,
}

impl std::fmt::Debug for Retrier<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Retrier")
            .field("policy", self.policy)
            .field("stop", &self.stop.reason())
            .finish()
    }
}

impl<'a> Retrier<'a> {
    /// A retrier with the given rules, spread and stop signal.
    pub fn new(
        policy: &'a RetryPolicy,
        jitter: &'a dyn JitterSource,
        stop: &'a StopSignal,
    ) -> Self {
        Self {
            policy,
            jitter,
            stop,
        }
    }

    /// The rules.
    pub fn policy(&self) -> &RetryPolicy {
        self.policy
    }

    /// The stop signal every wait of this retrier ends on.
    pub fn stop_signal(&self) -> &'a StopSignal {
        self.stop
    }

    /// Whether the fetch was told to stop.
    pub fn is_stopped(&self) -> bool {
        self.stop.is_stopped()
    }

    /// Calls `attempt` until it succeeds, fails for good or the stop signal
    /// fires, awaiting `before_attempt` before every attempt, the first one
    /// and every retry (261012-parallel-fetch functional-spec 3, U1:BR1.4).
    /// When `before_attempt` answers [`Permit::Stopped`], nothing is called
    /// and the result is Aborted, as for a stop during a retry wait
    /// (U1:BR1.5). The signal is checked before every attempt and during
    /// every wait. Retries, attempts and throttled attempts are recorded in
    /// `counter` (BR2.2, NFR2.4). The retry rules themselves are unchanged
    /// (NFR1.7).
    pub async fn call<T, B, BFut, F, Fut>(
        &self,
        counter: &mut RetryCounter,
        mut before_attempt: B,
        mut attempt: F,
    ) -> CallResult<T>
    where
        B: FnMut() -> BFut + Send,
        BFut: Future<Output = Permit> + Send,
        F: FnMut() -> Fut + Send,
        Fut: Future<Output = Result<T, ApiFailure>> + Send,
    {
        loop {
            if self.stop.is_stopped() || before_attempt().await == Permit::Stopped {
                return CallResult::Aborted;
            }
            let result = attempt().await;
            counter.record_attempt(result.as_ref().err().map(ApiFailure::kind));
            let failure = match result {
                Ok(value) => {
                    counter.record_success();
                    return CallResult::Succeeded(value);
                }
                Err(failure) => failure,
            };
            let jitter = self.jitter.sample(self.policy.jitter_ratio);
            match self
                .policy
                .decide(failure.kind(), counter.current_request(), jitter)
            {
                RetryDecision::GiveUp { exhausted } => {
                    return CallResult::Failed { failure, exhausted };
                }
                RetryDecision::RetryAfter(delay) => {
                    if wait_or_stop(delay, self.stop).await == WaitOutcome::Stopped {
                        return CallResult::Aborted;
                    }
                    counter.record_retry();
                }
            }
        }
    }

    /// [`Retrier::call`] without a wait before the attempts: the behaviour
    /// of the retrier before U1. Used for the calls that the per-region
    /// ceiling does not cover (`DescribeLogStreams`; FR2 covers
    /// `GetLogEvents` only).
    pub async fn call_unpaced<T, F, Fut>(
        &self,
        counter: &mut RetryCounter,
        attempt: F,
    ) -> CallResult<T>
    where
        F: FnMut() -> Fut + Send,
        Fut: Future<Output = Result<T, ApiFailure>> + Send,
    {
        self.call(counter, || std::future::ready(Permit::Proceed), attempt)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::failure::SafeDetailFields;

    fn failure(kind: FailureKind) -> ApiFailure {
        ApiFailure::new(kind, &SafeDetailFields::default())
    }

    fn secs(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    #[test]
    fn only_throttled_and_network_are_retried() {
        let policy = RetryPolicy::default();
        for kind in FailureKind::ALL {
            let expected = matches!(kind, FailureKind::Throttled | FailureKind::Network);
            assert_eq!(policy.is_retryable(kind), expected, "{kind:?}");
        }
        assert_eq!(
            policy.decide(FailureKind::AccessDenied, 0, 0.0),
            RetryDecision::GiveUp { exhausted: false }
        );
        assert_eq!(
            policy.decide(FailureKind::Network, 0, 0.0),
            RetryDecision::RetryAfter(secs(1))
        );
    }

    #[test]
    fn waits_double_from_one_to_sixteen_seconds() {
        let policy = RetryPolicy::default();
        let delays: Vec<_> = (1..=5).map(|n| policy.delay_before_retry(n, 0.0)).collect();
        assert_eq!(
            delays,
            vec![
                Some(secs(1)),
                Some(secs(2)),
                Some(secs(4)),
                Some(secs(8)),
                Some(secs(16))
            ]
        );
        assert_eq!(policy.delay_before_retry(6, 0.0), None);
        assert_eq!(policy.delay_before_retry(0, 0.0), None);
    }

    #[test]
    fn jitter_spreads_each_wait_by_at_most_twenty_percent() {
        let policy = RetryPolicy::default();
        assert_eq!(
            policy.delay_before_retry(1, -0.2),
            Some(Duration::from_millis(800))
        );
        assert_eq!(
            policy.delay_before_retry(1, 0.2),
            Some(Duration::from_millis(1_200))
        );
        assert_eq!(
            policy.delay_before_retry(5, -0.2),
            Some(Duration::from_millis(12_800))
        );
        assert_eq!(
            policy.delay_before_retry(5, 0.2),
            Some(Duration::from_millis(19_200))
        );
        // Values outside ±20% are clamped to the boundary.
        assert_eq!(
            policy.delay_before_retry(2, 0.9),
            Some(Duration::from_millis(2_400))
        );
        assert_eq!(
            policy.delay_before_retry(2, -0.9),
            Some(Duration::from_millis(1_600))
        );
        let random = RandomJitter;
        for _ in 0..1_000 {
            let value = random.sample(0.2);
            assert!((-0.2..=0.2).contains(&value), "{value}");
        }
        assert_eq!(FixedJitter(0.5).sample(0.2), 0.2);
    }

    #[test]
    fn five_retries_use_up_the_request() {
        let policy = RetryPolicy::default();
        for done in 0..5 {
            assert!(matches!(
                policy.decide(FailureKind::Throttled, done, 0.0),
                RetryDecision::RetryAfter(_)
            ));
        }
        assert_eq!(
            policy.decide(FailureKind::Throttled, 5, 0.0),
            RetryDecision::GiveUp { exhausted: true }
        );
    }

    #[test]
    fn a_success_restarts_the_count_for_the_next_request() {
        let mut counter = RetryCounter::new();
        counter.record_retry();
        counter.record_retry();
        assert_eq!(counter.current_request(), 2);
        counter.record_success();
        assert_eq!(counter.current_request(), 0);
        counter.record_retry();
        assert_eq!(counter.current_request(), 1);
        assert_eq!(counter.total(), 3, "the stream total spans requests");
    }

    #[tokio::test(start_paused = true)]
    async fn retrier_recovers_after_two_throttles_waiting_one_then_two_seconds() {
        let policy = RetryPolicy::default();
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let script = Mutex::new(vec![
            Err(failure(FailureKind::Throttled)),
            Err(failure(FailureKind::Throttled)),
            Ok(7),
        ]);
        let mut counter = RetryCounter::new();
        let started = tokio::time::Instant::now();
        let result = retrier
            .call_unpaced(&mut counter, || {
                let next = script.lock().unwrap().remove(0);
                async move { next }
            })
            .await;
        assert_eq!(result, CallResult::Succeeded(7));
        assert_eq!(counter.total(), 2);
        assert_eq!(counter.current_request(), 0);
        assert_eq!(started.elapsed(), secs(3));
    }

    #[tokio::test(start_paused = true)]
    async fn retrier_gives_up_after_six_calls_and_reports_exhaustion() {
        let policy = RetryPolicy::default();
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let calls = AtomicU32::new(0);
        let mut counter = RetryCounter::new();
        let result: CallResult<()> = retrier
            .call_unpaced(&mut counter, || {
                calls.fetch_add(1, Ordering::SeqCst);
                async { Err(failure(FailureKind::Network)) }
            })
            .await;
        assert_eq!(
            result,
            CallResult::Failed {
                failure: failure(FailureKind::Network),
                exhausted: true
            }
        );
        assert_eq!(calls.load(Ordering::SeqCst), 6);
        assert_eq!(counter.total(), 5);
    }

    #[tokio::test(start_paused = true)]
    async fn an_abort_during_the_wait_stops_without_calling_again() {
        let policy = RetryPolicy::default();
        let (handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let calls = AtomicU32::new(0);
        let mut counter = RetryCounter::new();
        let call = retrier.call_unpaced(&mut counter, || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Err::<(), _>(failure(FailureKind::Throttled)) }
        });
        let abort_later = async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            handle.stop(StopReason::Close);
        };
        let (result, ()) = tokio::join!(call, abort_later);
        assert_eq!(result, CallResult::Aborted);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    // ---- 261012-parallel-fetch U1: the stop signal with a reason (BR1.20) --

    #[test]
    fn a_first_stop_takes_the_reason_sent() {
        for reason in [StopReason::Cancel, StopReason::Close] {
            let (handle, signal) = stop_pair();
            assert_eq!(signal.reason(), None);
            assert!(!signal.is_stopped());
            handle.stop(reason);
            assert_eq!(signal.reason(), Some(reason));
            assert!(signal.is_stopped());
        }
    }

    #[test]
    fn close_after_cancel_becomes_close() {
        let (handle, signal) = stop_pair();
        handle.stop(StopReason::Cancel);
        handle.stop(StopReason::Close);
        assert_eq!(signal.reason(), Some(StopReason::Close));
    }

    #[test]
    fn cancel_after_close_stays_close() {
        let (handle, signal) = stop_pair();
        handle.stop(StopReason::Close);
        handle.stop(StopReason::Cancel);
        assert_eq!(signal.reason(), Some(StopReason::Close));
        assert_eq!(
            StopReason::stronger(Some(StopReason::Close), StopReason::Cancel),
            StopReason::Close
        );
    }

    #[test]
    fn the_same_reason_again_changes_nothing_and_wakes_no_one() {
        let (handle, signal) = stop_pair();
        assert!(
            handle.stop(StopReason::Cancel),
            "the first stop is a change"
        );
        assert!(!handle.stop(StopReason::Cancel), "the same reason is not");
        assert!(
            handle.stop(StopReason::Close),
            "Close over Cancel is a change"
        );
        let late = handle.signal();
        assert!(!handle.stop(StopReason::Close), "a repeated stop is not");
        assert!(!handle.stop(StopReason::Cancel), "a weaker stop is not");
        assert_eq!(late.receiver.has_changed().ok(), Some(false), "not woken");
        assert_eq!(signal.reason(), Some(StopReason::Close));
        assert_eq!(late.reason(), Some(StopReason::Close));
    }

    #[tokio::test(start_paused = true)]
    async fn stopped_completes_once_stopped() {
        let (handle, signal) = stop_pair();
        let stop_later = async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            handle.stop(StopReason::Close);
        };
        let started = tokio::time::Instant::now();
        let ((), ()) = tokio::join!(signal.stopped(), stop_later);
        assert_eq!(started.elapsed(), Duration::from_millis(300));
        assert_eq!(
            wait_or_stop(secs(10), &signal).await,
            WaitOutcome::Stopped,
            "an already stopped signal ends a wait at once"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_dropped_handle_or_never_does_not_stop() {
        let (handle, signal) = stop_pair();
        drop(handle);
        let never = StopSignal::never();
        assert!(!signal.is_stopped() && !never.is_stopped());
        assert_eq!(never.reason(), None);
        let waited = tokio::time::timeout(secs(60), signal.stopped()).await;
        assert!(waited.is_err(), "stopped() never completes");
        let waited = tokio::time::timeout(secs(60), never.stopped()).await;
        assert!(waited.is_err());
        assert_eq!(wait_or_stop(secs(1), &never).await, WaitOutcome::Elapsed);
    }

    #[tokio::test]
    async fn an_aborted_signal_prevents_the_first_call_and_never_fires_otherwise() {
        let policy = RetryPolicy::default();
        let (handle, signal) = stop_pair();
        handle.stop(StopReason::Close);
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let mut counter = RetryCounter::new();
        let result: CallResult<()> = retrier
            .call_unpaced(&mut counter, || async { panic!("must not be called") })
            .await;
        assert_eq!(result, CallResult::Aborted);
        assert!(handle.signal().is_stopped());
        assert!(!StopSignal::never().is_stopped());
    }

    // ---- 261012-parallel-fetch U1: the wait before every attempt ----------
    // (functional-spec 3, BR1.4, BR1.5, NFR1.7)

    /// Records "permit" and "call" in the order they happen.
    #[derive(Default)]
    struct Steps(Mutex<Vec<&'static str>>);

    impl Steps {
        fn push(&self, step: &'static str) {
            self.0.lock().unwrap().push(step);
        }

        fn take(&self) -> Vec<&'static str> {
            std::mem::take(&mut *self.0.lock().unwrap())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn call_waits_before_the_first_attempt_and_before_every_retry() {
        let policy = RetryPolicy::default();
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let steps = Steps::default();
        let script = Mutex::new(vec![
            Err(failure(FailureKind::Throttled)),
            Err(failure(FailureKind::Network)),
            Ok(3),
        ]);
        let mut counter = RetryCounter::new();
        let result = retrier
            .call(
                &mut counter,
                || {
                    steps.push("permit");
                    async { Permit::Proceed }
                },
                || {
                    steps.push("call");
                    let next = script.lock().unwrap().remove(0);
                    async move { next }
                },
            )
            .await;
        assert_eq!(result, CallResult::Succeeded(3));
        assert_eq!(
            steps.take(),
            ["permit", "call", "permit", "call", "permit", "call"]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_stopped_permit_ends_as_aborted_without_the_attempt() {
        let policy = RetryPolicy::default();
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let mut counter = RetryCounter::new();
        let first: CallResult<()> = retrier
            .call(
                &mut counter,
                || async { Permit::Stopped },
                || async { panic!("must not be called") },
            )
            .await;
        assert_eq!(first, CallResult::Aborted);

        // Stopped while waiting before a retry: one call only.
        let steps = Steps::default();
        let permits = Mutex::new(vec![Permit::Proceed, Permit::Stopped]);
        let retry: CallResult<()> = retrier
            .call(
                &mut counter,
                || {
                    let next = permits.lock().unwrap().remove(0);
                    async move { next }
                },
                || {
                    steps.push("call");
                    async { Err(failure(FailureKind::Throttled)) }
                },
            )
            .await;
        assert_eq!(retry, CallResult::Aborted);
        assert_eq!(steps.take(), ["call"]);
    }

    #[tokio::test(start_paused = true)]
    async fn the_retry_waits_and_count_are_unchanged_with_a_permit() {
        let policy = RetryPolicy::default();
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let call_times = Mutex::new(Vec::new());
        let started = tokio::time::Instant::now();
        let mut counter = RetryCounter::new();
        let result: CallResult<()> = retrier
            .call(
                &mut counter,
                || async { Permit::Proceed },
                || {
                    call_times.lock().unwrap().push(started.elapsed());
                    async { Err(failure(FailureKind::Throttled)) }
                },
            )
            .await;
        assert_eq!(
            result,
            CallResult::Failed {
                failure: failure(FailureKind::Throttled),
                exhausted: true
            }
        );
        assert_eq!(
            call_times.into_inner().unwrap(),
            [0, 1, 3, 7, 15, 31].map(secs),
            "1, 2, 4, 8 and 16 s between the six calls"
        );
        assert_eq!(counter.total(), 5);
    }

    #[tokio::test(start_paused = true)]
    async fn call_unpaced_behaves_like_call_with_an_immediate_permit() {
        let policy = RetryPolicy::default();
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let script = || {
            Mutex::new(vec![
                Err(failure(FailureKind::Throttled)),
                Err(failure(FailureKind::AccessDenied)),
            ])
        };
        let (unpaced_script, paced_script) = (script(), script());
        let (mut unpaced_counter, mut paced_counter) = (RetryCounter::new(), RetryCounter::new());
        let started = tokio::time::Instant::now();
        let unpaced: CallResult<()> = retrier
            .call_unpaced(&mut unpaced_counter, || {
                let next = unpaced_script.lock().unwrap().remove(0);
                async move { next }
            })
            .await;
        let unpaced_elapsed = started.elapsed();
        let paced: CallResult<()> = retrier
            .call(
                &mut paced_counter,
                || async { Permit::Proceed },
                || {
                    let next = paced_script.lock().unwrap().remove(0);
                    async move { next }
                },
            )
            .await;
        assert_eq!(unpaced, paced);
        assert_eq!(
            unpaced,
            CallResult::Failed {
                failure: failure(FailureKind::AccessDenied),
                exhausted: false
            }
        );
        assert_eq!(unpaced_counter, paced_counter);
        assert_eq!(unpaced_elapsed, secs(1));
        assert_eq!(started.elapsed(), secs(2));
    }

    #[tokio::test(start_paused = true)]
    async fn the_attempts_and_the_throttled_ones_are_counted() {
        let policy = RetryPolicy::default();
        let (_handle, signal) = stop_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let script = Mutex::new(vec![
            Err(failure(FailureKind::Throttled)),
            Err(failure(FailureKind::Network)),
            Err(failure(FailureKind::Throttled)),
            Ok(1),
            Err(failure(FailureKind::AccessDenied)),
        ]);
        let mut counter = RetryCounter::new();
        let next = || {
            let next = script.lock().unwrap().remove(0);
            async move { next }
        };
        let first = retrier
            .call(&mut counter, || async { Permit::Proceed }, next)
            .await;
        assert_eq!(first, CallResult::Succeeded(1));
        assert_eq!((counter.attempts(), counter.throttled()), (4, 2));
        let second = retrier.call_unpaced(&mut counter, next).await;
        assert!(matches!(second, CallResult::Failed { .. }));
        assert_eq!(
            (counter.attempts(), counter.throttled()),
            (5, 2),
            "the totals span requests, like the retry total"
        );
        assert_eq!(counter.total(), 3);
    }
}
