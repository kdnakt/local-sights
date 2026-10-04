//! U2 log group listing flow against a fake gateway. Nothing here connects
//! to AWS.

mod support;

use local_sights_core::catalog::ProfileSelector;
use local_sights_core::failure::{ApiFailure, FailureKind};
use local_sights_core::log_groups::listing::{
    ListingEnd, ListingRequest, ListingSink, run_listing,
};
use local_sights_core::log_groups::{
    EmptyState, ListingStatus, LogGroup, LogGroupListing, apply_failure_if_current,
    apply_page_if_current, empty_state,
};
use local_sights_core::paging::PageDecision;
use support::fake_gateway::{FakeGateway, failure, group_page};

/// Holds the current listing like AppSession does. Optionally replaces it
/// after a given number of pages, as a reload or connection change would.
struct SlotSink {
    current: Option<LogGroupListing>,
    replace_after_pages: Option<(usize, Option<LogGroupListing>)>,
    pages_seen: usize,
}

impl SlotSink {
    fn new(current: Option<LogGroupListing>) -> Self {
        Self {
            current,
            replace_after_pages: None,
            pages_seen: 0,
        }
    }

    fn listing(&self) -> &LogGroupListing {
        self.current.as_ref().expect("a current listing")
    }
}

impl ListingSink for SlotSink {
    fn is_current(&self, listing_id: &str) -> bool {
        self.current
            .as_ref()
            .is_some_and(|listing| listing.listing_id() == listing_id)
    }

    fn on_page(
        &mut self,
        listing_id: &str,
        groups: Vec<LogGroup>,
        next_token: Option<&str>,
    ) -> Option<PageDecision> {
        let decision = apply_page_if_current(self.current.as_mut(), listing_id, groups, next_token);
        self.pages_seen += 1;
        if let Some((after, _)) = &self.replace_after_pages {
            if *after == self.pages_seen {
                let (_, replacement) = self.replace_after_pages.take().unwrap();
                self.current = replacement;
            }
        }
        decision
    }

    fn on_failure(&mut self, listing_id: &str, failure: ApiFailure) -> bool {
        apply_failure_if_current(self.current.as_mut(), listing_id, failure)
    }
}

fn request(id: &str) -> ListingRequest {
    ListingRequest {
        listing_id: id.to_string(),
        profile: ProfileSelector::Named("dev".to_string()),
        region: "ap-northeast-1".to_string(),
    }
}

fn listing(id: &str) -> LogGroupListing {
    let request = request(id);
    LogGroupListing::start(request.listing_id, request.profile, request.region)
}

fn refs(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
}

fn names(listing: &LogGroupListing) -> Vec<String> {
    listing
        .groups()
        .iter()
        .map(|g| g.log_group_name.clone())
        .collect()
}

#[tokio::test]
async fn reads_every_page_and_sorts_more_than_a_hundred_groups() {
    let first: Vec<String> = (0..50).map(|i| format!("/app/{i:03}")).collect();
    let second: Vec<String> = (50..100).map(|i| format!("/app/{i:03}")).collect();
    let mut third = vec!["/aws/lambda/MyFunction", "/a-first"];
    let extra: Vec<String> = (100..110).map(|i| format!("/app/{i:03}")).collect();
    third.extend(refs(&extra));
    let gateway = FakeGateway::with_log_group_pages(vec![
        group_page(&refs(&second), Some("t1")),
        group_page(&refs(&first), Some("t2")),
        group_page(&third, None),
    ]);
    let mut sink = SlotSink::new(Some(listing("l1")));

    assert_eq!(
        run_listing(&gateway, &request("l1"), &mut sink).await,
        ListingEnd::Complete
    );

    let listing = sink.listing();
    assert_eq!(listing.status(), ListingStatus::Complete);
    assert_eq!(listing.groups().len(), 112);
    let sorted = names(listing);
    assert_eq!(sorted[0], "/a-first");
    assert_eq!(sorted[1], "/app/000");
    assert_eq!(
        sorted.last().map(String::as_str),
        Some("/aws/lambda/MyFunction")
    );
    let tokens: Vec<Option<String>> = gateway
        .group_calls()
        .into_iter()
        .map(|c| c.next_token)
        .collect();
    assert_eq!(
        tokens,
        vec![None, Some("t1".to_string()), Some("t2".to_string())]
    );
    for call in gateway.group_calls() {
        assert_eq!(call.profile_name.as_deref(), Some("dev"));
        assert_eq!(call.region.as_deref(), Some("ap-northeast-1"));
    }
}

#[tokio::test]
async fn an_empty_account_completes_with_no_groups() {
    let gateway = FakeGateway::with_log_group_pages(vec![group_page(&[], None)]);
    let mut sink = SlotSink::new(Some(listing("l1")));
    assert_eq!(
        run_listing(&gateway, &request("l1"), &mut sink).await,
        ListingEnd::Complete
    );
    assert_eq!(empty_state(sink.listing(), ""), Some(EmptyState::NoGroups));
}

#[tokio::test]
async fn an_error_on_the_first_page_is_partial_and_empty() {
    let gateway = FakeGateway::with_log_group_pages(vec![Err(failure(FailureKind::Network))]);
    let mut sink = SlotSink::new(Some(listing("l1")));
    assert_eq!(
        run_listing(&gateway, &request("l1"), &mut sink).await,
        ListingEnd::Partial
    );
    assert_eq!(sink.listing().status(), ListingStatus::Partial);
    assert!(sink.listing().groups().is_empty());
}

#[tokio::test]
async fn an_error_on_a_later_page_keeps_the_groups_read_so_far() {
    let gateway = FakeGateway::with_log_group_pages(vec![
        group_page(&["/b", "/a"], Some("t1")),
        Err(failure(FailureKind::Throttled)),
    ]);
    let mut sink = SlotSink::new(Some(listing("l1")));
    assert_eq!(
        run_listing(&gateway, &request("l1"), &mut sink).await,
        ListingEnd::Partial
    );
    assert_eq!(names(sink.listing()), ["/a", "/b"]);
    assert_eq!(
        sink.listing().failure().map(ApiFailure::kind),
        Some(FailureKind::Throttled)
    );
}

#[tokio::test]
async fn a_repeated_token_ends_the_listing() {
    let gateway = FakeGateway::with_log_group_pages(vec![
        group_page(&["/a"], Some("t1")),
        group_page(&["/b"], Some("t1")),
    ]);
    let mut sink = SlotSink::new(Some(listing("l1")));
    assert_eq!(
        run_listing(&gateway, &request("l1"), &mut sink).await,
        ListingEnd::Complete
    );
    assert_eq!(gateway.group_calls().len(), 2);
    assert_eq!(names(sink.listing()), ["/a", "/b"]);
}

#[tokio::test]
async fn a_superseded_listing_stops_and_late_pages_are_dropped() {
    let gateway = FakeGateway::with_log_group_pages(vec![group_page(&["/old"], Some("t1"))]);
    let mut sink = SlotSink::new(Some(listing("old")));
    // A reload replaces the current listing while the first page is applied.
    sink.replace_after_pages = Some((1, Some(listing("new"))));
    assert_eq!(
        run_listing(&gateway, &request("old"), &mut sink).await,
        ListingEnd::Superseded
    );
    assert_eq!(
        gateway.group_calls().len(),
        1,
        "the next page is not requested"
    );
    assert_eq!(sink.listing().listing_id(), "new");
    assert!(
        sink.listing().groups().is_empty(),
        "nothing leaked into the new listing"
    );

    // A late page for the old listing is dropped by the new current one.
    let late = FakeGateway::with_log_group_pages(vec![group_page(&["/late"], None)]);
    let mut stale = SlotSink::new(Some(listing("new")));
    assert_eq!(
        run_listing(&late, &request("old"), &mut stale).await,
        ListingEnd::Superseded
    );
    assert!(late.group_calls().is_empty());
    assert!(stale.listing().groups().is_empty());
}

#[tokio::test]
async fn no_current_listing_means_no_call() {
    let gateway = FakeGateway::with_log_group_pages(vec![]);
    let mut sink = SlotSink::new(None);
    assert_eq!(
        run_listing(&gateway, &request("l1"), &mut sink).await,
        ListingEnd::Superseded
    );
    assert!(gateway.group_calls().is_empty());
}

#[tokio::test]
async fn auth_and_permission_failures_do_not_stop_the_app() {
    for kind in [FailureKind::AuthRequired, FailureKind::AccessDenied] {
        let gateway = FakeGateway::with_log_group_pages(vec![Err(failure(kind))]);
        let mut sink = SlotSink::new(Some(listing("l1")));
        assert_eq!(
            run_listing(&gateway, &request("l1"), &mut sink).await,
            ListingEnd::Partial
        );
        assert_eq!(sink.listing().failure().map(ApiFailure::kind), Some(kind));
    }
}

#[tokio::test]
async fn sdk_default_profile_is_passed_as_no_profile() {
    let gateway = FakeGateway::with_log_group_pages(vec![group_page(&[], None)]);
    let mut request = request("l1");
    request.profile = ProfileSelector::SdkDefault;
    let mut sink = SlotSink::new(Some(LogGroupListing::start(
        "l1".to_string(),
        ProfileSelector::SdkDefault,
        "us-east-1".to_string(),
    )));
    run_listing(&gateway, &request, &mut sink).await;
    assert_eq!(gateway.group_calls()[0].profile_name, None);
}
