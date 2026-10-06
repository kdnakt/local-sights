//! U5 fetch and log filter together against a fake gateway: pages that
//! arrive while a filter is in force are judged as they are added (FR6.4,
//! BR2.1), a new fetch empties the result and keeps the text (BR2.2), a scan
//! that overlaps arriving pages misses nothing and counts nothing twice
//! (BR2.1, review R-01), and work of an older timeline is dropped (BR2.4).
//! The fetch reaches the LogView as its `TimelineStore`, so each page and
//! its judgement happen under one lock (BR2.5). Waits run on tokio's paused
//! clock, and nothing here connects to AWS.

mod support;

use std::sync::Mutex;

use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink, JobStatus, run_fetch};
use local_sights_core::fetcher::StreamFetchOutcome;
use local_sights_core::filter::{FilterChange, FilterStatus, ScanStep, ScanTicket};
use local_sights_core::log_view::LogView;
use local_sights_core::request::{FetchInput, ValidatedFetch, validate_fetch_input};
use local_sights_core::retry::{FixedJitter, Retrier, RetryPolicy, abort_pair};
use local_sights_core::streams::planner::ListingProgress;
use support::fake_gateway::{FakeGateway, page, stream_page};
use support::recording_sink::RecordingSink;

/// 2024-01-02 03:04:05 UTC in epoch milliseconds (start of the range).
const START: i64 = 1_704_164_645_000;

fn validated() -> ValidatedFetch {
    validate_fetch_input(&FetchInput {
        profile_name: "dev".to_string(),
        log_group_name: "/aws/lambda/orders".to_string(),
        start_text: "2024-01-02 03:04:05".to_string(),
        end_text: "2024-01-02 03:05:05".to_string(),
    })
    .expect("test input is valid")
}

async fn fetch_into<S: FetchSink + Send>(
    gateway: &FakeGateway,
    view: &Mutex<LogView>,
    sink: &mut S,
) -> FetchJob {
    let (_handle, signal) = abort_pair();
    let policy = RetryPolicy::default();
    let jitter = FixedJitter(0.0);
    let retrier = Retrier::new(&policy, &jitter, &signal);
    run_fetch(gateway, &validated(), view, &retrier, sink).await
}

async fn fetch(gateway: &FakeGateway, view: &Mutex<LogView>) -> FetchJob {
    fetch_into(gateway, view, &mut RecordingSink::default()).await
}

/// Two streams; the messages that contain "error" in any case are `a0`,
/// `b1` (on its second line) and `a3`.
fn two_streams() -> FakeGateway {
    FakeGateway::with_stream_pages(vec![stream_page(
        &[("a", None, None), ("b", None, None)],
        None,
    )])
    .script_stream(
        "a",
        vec![
            page(&[(START, "a0 ERROR"), (START + 4, "a1 ok")], Some("f/1")),
            page(
                &[(START + 8, "a2 ok"), (START + 9, "a3 Error")],
                Some("f/1"),
            ),
        ],
    )
    .script_stream(
        "b",
        vec![page(
            &[(START + 1, "b0 ok"), (START + 5, "b1\nsecond line error")],
            None,
        )],
    )
}

/// `(stream, message)` of the rows the screen would get from offset 0.
fn shown(view: &Mutex<LogView>) -> Vec<(String, String)> {
    view.lock()
        .unwrap()
        .rows(0, 1_000)
        .rows
        .into_iter()
        .map(|e| (e.log_stream_name, e.message))
        .collect()
}

fn pair(stream: &str, message: &str) -> (String, String) {
    (stream.to_string(), message.to_string())
}

/// The held events whose message contains `needle` in any case, in order.
fn expected(view: &Mutex<LogView>, needle: &str) -> Vec<(String, String)> {
    view.lock()
        .unwrap()
        .timeline()
        .events()
        .iter()
        .filter(|e| e.message.to_lowercase().contains(needle))
        .map(|e| (e.log_stream_name.clone(), e.message.clone()))
        .collect()
}

fn set_filter(view: &Mutex<LogView>, text: &str) -> Option<ScanTicket> {
    match view.lock().unwrap().set_filter(text) {
        FilterChange::Started(ticket) => Some(ticket),
        _ => None,
    }
}

fn finish_scan(view: &Mutex<LogView>, ticket: ScanTicket) {
    while view.lock().unwrap().scan_chunk(ticket, 2) == ScanStep::Progressed {}
}

#[tokio::test(start_paused = true)]
async fn with_a_filter_in_force_only_the_matching_rows_of_arriving_pages_are_shown() {
    let view = Mutex::new(LogView::new());
    let ticket = set_filter(&view, "error").unwrap();
    finish_scan(&view, ticket);
    let gateway = two_streams();
    let job = fetch(&gateway, &view).await;
    assert_eq!(job.status, JobStatus::Completed);

    assert_eq!(
        shown(&view),
        vec![
            pair("a", "a0 ERROR"),
            pair("b", "b1\nsecond line error"),
            pair("a", "a3 Error"),
        ]
    );
    let state = view.lock().unwrap().filter_state();
    let summary = state.summary.unwrap();
    assert_eq!((summary.matched_count, summary.all_count), (3, 6));
    assert_eq!(summary.status, FilterStatus::Ready);

    // FR6.2: filtering again calls no API.
    let calls = gateway.api_log().len();
    let ticket = set_filter(&view, "OK").unwrap();
    finish_scan(&view, ticket);
    assert_eq!(gateway.api_log().len(), calls);
    assert_eq!(shown(&view).len(), 3);
}

#[tokio::test(start_paused = true)]
async fn a_new_fetch_empties_the_result_and_filters_the_new_pages_with_the_same_text() {
    let view = Mutex::new(LogView::new());
    fetch(&two_streams(), &view).await;
    let ticket = set_filter(&view, "ERROR").unwrap();
    finish_scan(&view, ticket);
    assert_eq!(shown(&view).len(), 3);
    let epoch = view.lock().unwrap().filter().timeline_epoch();

    let second = FakeGateway::with_stream_pages(vec![stream_page(&[("c", None, None)], None)])
        .script_stream(
            "c",
            vec![page(
                &[(START + 2, "c0 error again"), (START + 3, "c1 fine")],
                None,
            )],
        );
    fetch(&second, &view).await;

    assert_eq!(shown(&view), vec![pair("c", "c0 error again")]);
    let guard = view.lock().unwrap();
    assert_eq!(guard.filter().condition().text, "ERROR", "the text stays");
    assert_eq!(guard.filter().timeline_epoch(), epoch + 1);
    let summary = guard.filter_state().summary.unwrap();
    assert_eq!((summary.matched_count, summary.all_count), (1, 2));
}

/// Changes the filter after the first page, then judges one small chunk
/// after every later page, so the scan and the arriving pages overlap.
struct ScanningSink<'a> {
    view: &'a Mutex<LogView>,
    ticket: Option<ScanTicket>,
    batches: usize,
    scanned_chunks: usize,
}

impl FetchSink for ScanningSink<'_> {
    fn on_started(&mut self, _job: &FetchJob, _timeline_version: u64) {}
    fn on_listing_progress(&mut self, _job_id: u64, _progress: ListingProgress) {}
    fn on_planned(&mut self, _job: &FetchJob) {}

    fn on_batch(&mut self, _job_id: u64, _progress: BatchProgress) {
        self.batches += 1;
        match self.ticket {
            None => self.ticket = set_filter(self.view, "m"),
            Some(ticket) => {
                if self.view.lock().unwrap().scan_chunk(ticket, 2) != ScanStep::Stale {
                    self.scanned_chunks += 1;
                }
            }
        }
    }

    fn on_stream_finished(&mut self, _job: &FetchJob, _outcome: &StreamFetchOutcome) {}
    fn on_finished(&mut self, _job: &FetchJob, _timeline_version: u64) {}
}

#[tokio::test(start_paused = true)]
async fn pages_arriving_during_a_scan_are_neither_missed_nor_counted_twice() {
    // Stream "x" first, then "y", whose times interleave with x's, so later
    // pages land both before and after the scan cursor. Each stream has four
    // pages; the fourth repeats the third token, which ends the stream.
    let pages = |stream: &str, offset: i64, message: fn(i64) -> &'static str| {
        let tokens = ["/0", "/1", "/2", "/2"];
        (0..4i64)
            .map(|p| {
                let events: Vec<(i64, &str)> = (0..4i64)
                    .map(|i| (START + (p * 4 + i) * 10 + offset, message(i)))
                    .collect();
                page(&events, Some(&format!("{stream}{}", tokens[p as usize])))
            })
            .collect::<Vec<_>>()
    };
    let gateway = FakeGateway::with_stream_pages(vec![stream_page(
        &[("x", None, None), ("y", None, None)],
        None,
    )])
    .script_stream(
        "x",
        pages("x", 0, |i| if i % 2 == 0 { "m hit" } else { "other" }),
    )
    .script_stream("y", pages("y", 5, |_| "M y"));

    let view = Mutex::new(LogView::new());
    let mut sink = ScanningSink {
        view: &view,
        ticket: None,
        batches: 0,
        scanned_chunks: 0,
    };
    let job = fetch_into(&gateway, &view, &mut sink).await;
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(sink.batches, 8);
    assert!(sink.scanned_chunks >= 3, "the scan overlapped the pages");
    let ticket = sink.ticket.unwrap();
    assert_eq!(
        view.lock().unwrap().filter().status(),
        FilterStatus::Filtering,
        "still scanning when the fetch ends"
    );
    finish_scan(&view, ticket);

    let rows = shown(&view);
    assert_eq!(rows, expected(&view, "m"));
    assert_eq!(rows.len(), 8 + 16, "8 hits of x and all 16 of y");
    let summary = view.lock().unwrap().filter_state().summary.unwrap();
    assert_eq!((summary.matched_count, summary.all_count), (24, 32));
}

#[tokio::test(start_paused = true)]
async fn a_scan_of_a_discarded_timeline_stops_and_adds_nothing() {
    let view = Mutex::new(LogView::new());
    fetch(&two_streams(), &view).await;
    let ticket = set_filter(&view, "error").unwrap();
    assert_eq!(
        view.lock().unwrap().scan_chunk(ticket, 2),
        ScanStep::Progressed
    );

    // A new fetch discards the timeline while the scan is half-way.
    let second = FakeGateway::with_stream_pages(vec![stream_page(&[("c", None, None)], None)])
        .script_stream("c", vec![page(&[(START, "c0 Error")], None)]);
    fetch(&second, &view).await;

    assert_eq!(view.lock().unwrap().scan_chunk(ticket, 2), ScanStep::Stale);
    assert_eq!(shown(&view), vec![pair("c", "c0 Error")]);
    let summary = view.lock().unwrap().filter_state().summary.unwrap();
    assert_eq!(summary.status, FilterStatus::Ready);
    assert_eq!((summary.matched_count, summary.all_count), (1, 1));
}
