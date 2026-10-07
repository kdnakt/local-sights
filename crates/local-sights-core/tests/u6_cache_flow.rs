//! U6 fetch and disk cache together against a fake gateway and a temporary
//! folder: a fetch is written and the same range fetched again is served
//! from the cache without any API call, with the U5 log filter applied
//! (FR7.4, BR2.3); a following range is fetched from AWS and joins the
//! cached range, after which the joined range is served from the cache too
//! (FR7.5, BR3.3); and the part newer than five minutes before the fetch
//! started is never recorded, so a range reaching into it goes to AWS again
//! (FR7.9, BR3.1). Nothing here connects to AWS or touches the user's
//! folders.

mod support;

use std::sync::Mutex;

use local_sights_core::cache::plan::{CacheKey, CacheOutcome, CoveredRange};
use local_sights_core::cache::{HeaderRead, LogCache};
use local_sights_core::catalog::ProfileSelector;
use local_sights_core::coordinator::{FetchJob, JobStatus, run_fetch_with_cache};
use local_sights_core::filter::{FilterChange, ScanStep};
use local_sights_core::log_view::LogView;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::retry::{FixedJitter, Retrier, RetryPolicy, abort_pair};
use support::fake_gateway::{FakeGateway, page, stream_page};
use support::recording_sink::RecordingSink;

/// 2024-01-02 03:04:05 UTC in epoch milliseconds.
const T_030405: i64 = 1_704_164_645_000;
/// One minute in milliseconds.
const MINUTE: i64 = 60_000;
/// A fetch started a day later: everything fetched is recordable.
const A_DAY_LATER: i64 = T_030405 + 1_440 * MINUTE;

/// `dev` in ap-northeast-1, `/aws/lambda/orders`, from `start` to `end`
/// (`yyyy-mm-dd hh:mm:ss` UTC, the end second included).
fn validated(start: &str, end: &str) -> ValidatedFetch {
    let validated = validate_fetch_input(&FetchInput {
        profile_name: "dev".to_string(),
        log_group_name: "/aws/lambda/orders".to_string(),
        start_text: start.to_string(),
        end_text: end.to_string(),
    })
    .expect("test input is valid");
    ValidatedFetch {
        request: validated.request.with_connection(
            ProfileSelector::Named("dev".to_string()),
            "ap-northeast-1".to_string(),
        ),
        range: validated.range,
    }
}

async fn fetch(
    gateway: &FakeGateway,
    view: &Mutex<LogView>,
    cache: &LogCache,
    request: &ValidatedFetch,
    started_at: i64,
) -> FetchJob {
    let (_handle, signal) = abort_pair();
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(0.0);
    let retrier = Retrier::new(&policy, &jitter, &signal);
    let mut sink = RecordingSink::default();
    run_fetch_with_cache(
        gateway,
        request,
        view,
        &retrier,
        Some(cache),
        started_at,
        &mut sink,
    )
    .await
}

/// The gateway answers one listing of stream `a` per fetch, and `pages` of
/// `a` in order, one page per fetch.
fn gateway(fetches: usize, pages: Vec<Vec<(i64, &str)>>) -> FakeGateway {
    let listings = (0..fetches)
        .map(|_| stream_page(&[("a", None, None)], None))
        .collect();
    FakeGateway::with_stream_pages(listings)
        .script_stream("a", pages.iter().map(|events| page(events, None)).collect())
}

fn messages(view: &Mutex<LogView>) -> Vec<String> {
    view.lock()
        .unwrap()
        .timeline()
        .events()
        .iter()
        .map(|event| event.message.clone())
        .collect()
}

fn covered(cache: &LogCache, request: &ValidatedFetch) -> Vec<CoveredRange> {
    let key = CacheKey::from_request(&request.request).unwrap();
    match cache.read_header(&key) {
        HeaderRead::Found(header) => header.covered_ranges,
        other => panic!("expected a cache header, got {other:?}"),
    }
}

#[tokio::test]
async fn the_same_range_again_is_served_from_the_cache_without_aws_and_filtered() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LogCache::new(dir.path().join("cache"));
    let request = validated("2024-01-02 03:04:05", "2024-01-02 03:05:05");
    let gateway = gateway(
        1,
        vec![vec![
            (T_030405, "a0 ERROR"),
            (T_030405 + 10, "a1 ok"),
            (T_030405 + 20, "a2 error again"),
        ]],
    );
    let view = Mutex::new(LogView::new());
    let first = fetch(&gateway, &view, &cache, &request, A_DAY_LATER).await;
    assert_eq!(first.cache_outcome, CacheOutcome::Saved);
    let calls = gateway.api_log().len();
    assert_eq!(calls, 2, "one listing and one page");

    // U5: a filter in force before the fetch applies to the cached rows.
    let ticket = match view.lock().unwrap().set_filter("error") {
        FilterChange::Started(ticket) => ticket,
        other => panic!("expected a scan, got {other:?}"),
    };
    while view.lock().unwrap().scan_chunk(ticket, 2) == ScanStep::Progressed {}

    let second = fetch(&gateway, &view, &cache, &request, A_DAY_LATER).await;
    assert_eq!(gateway.api_log().len(), calls, "no API call at all");
    assert_eq!(second.status, JobStatus::Completed);
    assert!(second.served_from_cache);
    assert_eq!(second.cache_outcome, CacheOutcome::Hit);
    assert_eq!(second.event_count, 3);
    assert_eq!(messages(&view), ["a0 ERROR", "a1 ok", "a2 error again"]);
    let shown: Vec<String> = view
        .lock()
        .unwrap()
        .rows(0, 100)
        .rows
        .into_iter()
        .map(|event| event.message)
        .collect();
    assert_eq!(shown, ["a0 ERROR", "a2 error again"], "the filter applies");
}

#[tokio::test]
async fn a_following_range_joins_the_cached_one_and_the_joined_range_hits() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LogCache::new(dir.path().join("cache"));
    let first = validated("2024-01-02 03:04:05", "2024-01-02 03:05:05");
    let next = validated("2024-01-02 03:05:06", "2024-01-02 03:06:05");
    let joined = validated("2024-01-02 03:04:05", "2024-01-02 03:06:05");
    let gateway = gateway(
        2,
        vec![
            vec![(T_030405 + 1, "first minute")],
            vec![(T_030405 + MINUTE + 1_001, "second minute")],
        ],
    );
    let view = Mutex::new(LogView::new());

    let job = fetch(&gateway, &view, &cache, &first, A_DAY_LATER).await;
    assert_eq!(job.cache_outcome, CacheOutcome::Saved);
    // The next minute is not cached: it goes to AWS and is saved.
    let job = fetch(&gateway, &view, &cache, &next, A_DAY_LATER).await;
    assert_eq!(job.cache_outcome, CacheOutcome::Saved);
    assert_eq!(gateway.api_log().len(), 4, "two listings and two pages");
    assert_eq!(
        covered(&cache, &joined),
        vec![CoveredRange {
            start_ms: T_030405,
            end_ms: T_030405 + 2 * MINUTE + 999,
        }],
        "adjacent ranges are joined into one"
    );

    let job = fetch(&gateway, &view, &cache, &joined, A_DAY_LATER).await;
    assert_eq!(job.cache_outcome, CacheOutcome::Hit);
    assert_eq!(messages(&view), ["first minute", "second minute"]);
    let job = fetch(&gateway, &view, &cache, &first, A_DAY_LATER).await;
    assert_eq!(
        job.cache_outcome,
        CacheOutcome::Hit,
        "a part of it hits too"
    );
    assert_eq!(messages(&view), ["first minute"]);
    assert_eq!(gateway.api_log().len(), 4, "no further API call");
}

#[tokio::test]
async fn the_last_five_minutes_before_the_fetch_are_not_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let cache = LogCache::new(dir.path().join("cache"));
    // Ten minutes fetched at 03:16:05, two minutes after their end: only
    // the part up to five minutes before that, 03:11:05.000, is recorded.
    let ten_minutes = validated("2024-01-02 03:04:05", "2024-01-02 03:14:05");
    let started_at = T_030405 + 12 * MINUTE;
    let gateway = gateway(
        2,
        vec![
            vec![
                (T_030405 + MINUTE, "old enough"),
                (T_030405 + 9 * MINUTE, "too new"),
            ],
            vec![
                (T_030405 + MINUTE, "old enough"),
                (T_030405 + 9 * MINUTE, "too new"),
            ],
        ],
    );
    let view = Mutex::new(LogView::new());

    let job = fetch(&gateway, &view, &cache, &ten_minutes, started_at).await;
    assert_eq!(job.cache_outcome, CacheOutcome::Saved);
    assert_eq!(
        covered(&cache, &ten_minutes),
        vec![CoveredRange {
            start_ms: T_030405,
            end_ms: started_at - 5 * MINUTE,
        }]
    );

    let older = validated("2024-01-02 03:04:05", "2024-01-02 03:09:05");
    let job = fetch(&gateway, &view, &cache, &older, started_at).await;
    assert_eq!(job.cache_outcome, CacheOutcome::Hit);
    assert_eq!(messages(&view), ["old enough"]);
    assert_eq!(gateway.api_log().len(), 2);

    // The whole ten minutes reach past the recorded part: AWS again.
    let job = fetch(&gateway, &view, &cache, &ten_minutes, started_at).await;
    assert!(!job.served_from_cache);
    assert_eq!(gateway.api_log().len(), 4);
    assert_eq!(messages(&view), ["old enough", "too new"]);
}
