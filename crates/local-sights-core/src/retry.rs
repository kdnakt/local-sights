//! RetryPolicy: which failures are retried, how long to wait and when to
//! give up (U3:BR2.1-BR2.3), plus the abort signal used by every wait.
//!
//! The decisions are pure functions; [`Retrier`] runs them around one API
//! call, waiting with `tokio::time::sleep` and stopping as soon as the
//! [`AbortSignal`] fires. The random spread of each wait comes from a
//! [`JitterSource`], so tests can fix it.

use std::future::Future;
use std::time::Duration;

use tokio::sync::watch;

use crate::failure::{ApiFailure, FailureKind};

/// Rules of retrying (entities.md RetryPolicy, Q1).
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// Wait before the n-th retry, in milliseconds (1, 2, 4, 8, 16 s).
    pub base_delays_ms: Vec<u64>,
    /// Each wait is spread by up to this ratio either way (0.2 = ±20%).
    pub jitter_ratio: f64,
    /// Retries per request; the first call plus this many at most.
    pub max_retries: u32,
    /// Streams that exhaust their retries in a row before the remaining
    /// streams are failed without calling the API (review R-03).
    pub max_consecutive_exhausted: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_delays_ms: vec![1_000, 2_000, 4_000, 8_000, 16_000],
            jitter_ratio: 0.2,
            max_retries: 5,
            max_consecutive_exhausted: 3,
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

/// Counts retries per request and in total for one stream (BR2.2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetryCounter {
    current_request: u32,
    total: u32,
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
}

/// How one stream ended, as far as the exhaustion streak is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamEnding {
    /// Read to the last page.
    Succeeded,
    /// Failed without using up the retries (e.g. AccessDenied).
    FailedWithoutExhaustion,
    /// Failed after using up every retry.
    Exhausted,
}

/// Counts streams that exhausted their retries in a row (BR2.1, R-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExhaustionStreak {
    consecutive: u32,
    limit: u32,
}

impl ExhaustionStreak {
    /// A streak that trips after `limit` exhausted streams in a row.
    pub fn new(limit: u32) -> Self {
        Self {
            consecutive: 0,
            limit: limit.max(1),
        }
    }

    /// Records how a stream ended; anything but Exhausted resets the count.
    pub fn record(&mut self, ending: StreamEnding) {
        self.consecutive = match ending {
            StreamEnding::Exhausted => self.consecutive.saturating_add(1),
            StreamEnding::Succeeded | StreamEnding::FailedWithoutExhaustion => 0,
        };
    }

    /// Number of exhausted streams in a row so far.
    pub fn consecutive(&self) -> u32 {
        self.consecutive
    }

    /// Whether the remaining streams must be failed without calling.
    pub fn limit_reached(&self) -> bool {
        self.consecutive >= self.limit
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

/// The caller's side of an abort signal.
#[derive(Debug)]
pub struct AbortHandle {
    sender: watch::Sender<bool>,
}

/// The fetch's side of an abort signal (BR2.3, BR5.5).
#[derive(Debug, Clone)]
pub struct AbortSignal {
    receiver: watch::Receiver<bool>,
}

/// A connected handle and signal.
pub fn abort_pair() -> (AbortHandle, AbortSignal) {
    let (sender, receiver) = watch::channel(false);
    (AbortHandle { sender }, AbortSignal { receiver })
}

impl AbortHandle {
    /// Tells the fetch to stop. Calling it again does nothing more.
    pub fn abort(&self) {
        self.sender.send_replace(true);
    }

    /// A new signal connected to this handle.
    pub fn signal(&self) -> AbortSignal {
        AbortSignal {
            receiver: self.sender.subscribe(),
        }
    }
}

impl AbortSignal {
    /// A signal that never fires (the `fetch_check` example).
    pub fn never() -> Self {
        let (handle, signal) = abort_pair();
        // Dropping the handle closes the channel; `aborted` then never
        // completes and `is_aborted` stays false.
        drop(handle);
        signal
    }

    /// Whether the abort was requested.
    pub fn is_aborted(&self) -> bool {
        *self.receiver.borrow()
    }

    /// Completes when the abort is requested; never, when the handle was
    /// dropped without aborting.
    pub async fn aborted(&self) {
        let mut receiver = self.receiver.clone();
        if receiver.wait_for(|aborted| *aborted).await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// How a wait ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitOutcome {
    /// The whole delay passed.
    Elapsed,
    /// The abort was requested first (BR2.3).
    Aborted,
}

/// Waits `delay` unless the abort is requested first.
pub async fn wait_or_abort(delay: Duration, signal: &AbortSignal) -> WaitOutcome {
    if signal.is_aborted() {
        return WaitOutcome::Aborted;
    }
    tokio::select! {
        () = tokio::time::sleep(delay) => WaitOutcome::Elapsed,
        () = signal.aborted() => WaitOutcome::Aborted,
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
    /// The abort was requested; the request was not sent again.
    Aborted,
}

/// Runs one request with the retry rules and the abort signal.
#[derive(Clone, Copy)]
pub struct Retrier<'a> {
    policy: &'a RetryPolicy,
    jitter: &'a dyn JitterSource,
    abort: &'a AbortSignal,
}

impl std::fmt::Debug for Retrier<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Retrier")
            .field("policy", self.policy)
            .field("aborted", &self.abort.is_aborted())
            .finish()
    }
}

impl<'a> Retrier<'a> {
    /// A retrier with the given rules, spread and abort signal.
    pub fn new(
        policy: &'a RetryPolicy,
        jitter: &'a dyn JitterSource,
        abort: &'a AbortSignal,
    ) -> Self {
        Self {
            policy,
            jitter,
            abort,
        }
    }

    /// The rules.
    pub fn policy(&self) -> &RetryPolicy {
        self.policy
    }

    /// Whether the abort was requested.
    pub fn is_aborted(&self) -> bool {
        self.abort.is_aborted()
    }

    /// Calls `call` until it succeeds, fails for good or the abort is
    /// requested. The abort is checked before every call and during every
    /// wait. Retries are recorded in `counter` (BR2.2).
    pub async fn call<T, F, Fut>(&self, counter: &mut RetryCounter, mut call: F) -> CallResult<T>
    where
        F: FnMut() -> Fut + Send,
        Fut: Future<Output = Result<T, ApiFailure>> + Send,
    {
        loop {
            if self.abort.is_aborted() {
                return CallResult::Aborted;
            }
            let failure = match call().await {
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
                    if wait_or_abort(delay, self.abort).await == WaitOutcome::Aborted {
                        return CallResult::Aborted;
                    }
                    counter.record_retry();
                }
            }
        }
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

    #[test]
    fn three_exhausted_streams_in_a_row_trip_the_limit() {
        let mut streak = ExhaustionStreak::new(RetryPolicy::default().max_consecutive_exhausted);
        streak.record(StreamEnding::Exhausted);
        streak.record(StreamEnding::Exhausted);
        assert!(!streak.limit_reached());
        streak.record(StreamEnding::Exhausted);
        assert!(streak.limit_reached());
        assert_eq!(streak.consecutive(), 3);
    }

    #[test]
    fn a_success_or_a_non_exhausting_failure_resets_the_streak() {
        let mut streak = ExhaustionStreak::new(3);
        streak.record(StreamEnding::Exhausted);
        streak.record(StreamEnding::Exhausted);
        streak.record(StreamEnding::Succeeded);
        assert_eq!(streak.consecutive(), 0);
        streak.record(StreamEnding::Exhausted);
        streak.record(StreamEnding::Exhausted);
        streak.record(StreamEnding::FailedWithoutExhaustion);
        assert_eq!(streak.consecutive(), 0);
        streak.record(StreamEnding::Exhausted);
        assert!(!streak.limit_reached());
    }

    #[tokio::test(start_paused = true)]
    async fn retrier_recovers_after_two_throttles_waiting_one_then_two_seconds() {
        let policy = RetryPolicy::default();
        let (_handle, signal) = abort_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let script = Mutex::new(vec![
            Err(failure(FailureKind::Throttled)),
            Err(failure(FailureKind::Throttled)),
            Ok(7),
        ]);
        let mut counter = RetryCounter::new();
        let started = tokio::time::Instant::now();
        let result = retrier
            .call(&mut counter, || {
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
        let (_handle, signal) = abort_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let calls = AtomicU32::new(0);
        let mut counter = RetryCounter::new();
        let result: CallResult<()> = retrier
            .call(&mut counter, || {
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
        let (handle, signal) = abort_pair();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let calls = AtomicU32::new(0);
        let mut counter = RetryCounter::new();
        let call = retrier.call(&mut counter, || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Err::<(), _>(failure(FailureKind::Throttled)) }
        });
        let abort_later = async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            handle.abort();
        };
        let (result, ()) = tokio::join!(call, abort_later);
        assert_eq!(result, CallResult::Aborted);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn an_aborted_signal_prevents_the_first_call_and_never_fires_otherwise() {
        let policy = RetryPolicy::default();
        let (handle, signal) = abort_pair();
        handle.abort();
        let retrier = Retrier::new(&policy, &FixedJitter(0.0), &signal);
        let mut counter = RetryCounter::new();
        let result: CallResult<()> = retrier
            .call(&mut counter, || async { panic!("must not be called") })
            .await;
        assert_eq!(result, CallResult::Aborted);
        assert!(handle.signal().is_aborted());
        assert!(!AbortSignal::never().is_aborted());
    }
}
