//! LogGroupBrowser: the log group list of one connection (U2).
//!
//! Pure logic: paging decisions (BR3.1), sorting (BR3.3), partial results
//! (BR3.4), discarding stale responses (BR3.6), filtering (BR3.7) and the
//! empty states (BR3.10). The page-by-page flow is in [`listing`].

pub mod listing;

use serde::Serialize;

use crate::catalog::ProfileSelector;
use crate::failure::ApiFailure;
use crate::paging::{PageDecision, decide};

/// One log group of the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogGroup {
    /// Log group name.
    pub log_group_name: String,
    /// ARN; not shown and not sent to the screen (contains the account ID).
    #[serde(skip)]
    pub arn: Option<String>,
    /// Creation time, epoch milliseconds.
    pub creation_time: Option<i64>,
}

/// Progress of a listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ListingStatus {
    /// Pages are still being read.
    Loading,
    /// Every page was read.
    Complete,
    /// Stopped by an error; holds the groups read so far.
    Partial,
}

/// Why an empty list is empty (BR3.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EmptyState {
    /// The connection has no log groups.
    NoGroups,
    /// The filter matches none of the groups.
    NoMatches,
}

/// The log group list of one connection, identified by `listing_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogGroupListing {
    listing_id: String,
    profile: ProfileSelector,
    region: String,
    status: ListingStatus,
    groups: Vec<LogGroup>,
    failure: Option<ApiFailure>,
    sent_token: Option<String>,
}

impl LogGroupListing {
    /// A new listing in status Loading.
    pub fn start(listing_id: String, profile: ProfileSelector, region: String) -> Self {
        Self {
            listing_id,
            profile,
            region,
            status: ListingStatus::Loading,
            groups: Vec::new(),
            failure: None,
            sent_token: None,
        }
    }

    /// ID of this listing.
    pub fn listing_id(&self) -> &str {
        &self.listing_id
    }

    /// Profile of the connection.
    pub fn profile(&self) -> &ProfileSelector {
        &self.profile
    }

    /// Region of the connection.
    pub fn region(&self) -> &str {
        &self.region
    }

    /// Current status.
    pub fn status(&self) -> ListingStatus {
        self.status
    }

    /// Groups read so far, by name ascending (BR3.3).
    pub fn groups(&self) -> &[LogGroup] {
        &self.groups
    }

    /// The failure, when Partial.
    pub fn failure(&self) -> Option<&ApiFailure> {
        self.failure.as_ref()
    }

    /// Whether a group with this name has been listed.
    pub fn contains(&self, log_group_name: &str) -> bool {
        self.groups
            .iter()
            .any(|group| group.log_group_name == log_group_name)
    }

    /// Adds one page and decides whether to read the next one: no token, or
    /// the token that was just sent, ends the listing (BR3.1).
    pub fn apply_page(&mut self, groups: Vec<LogGroup>, next_token: Option<&str>) -> PageDecision {
        self.groups.extend(groups);
        self.groups
            .sort_by(|a, b| a.log_group_name.cmp(&b.log_group_name));
        let decision = decide(self.sent_token.as_deref(), next_token);
        match decision {
            PageDecision::Continue => self.sent_token = next_token.map(str::to_string),
            PageDecision::Finished => self.status = ListingStatus::Complete,
        }
        decision
    }

    /// Stops the listing as Partial, keeping the groups read so far (BR3.4).
    pub fn apply_failure(&mut self, failure: ApiFailure) {
        self.status = ListingStatus::Partial;
        self.failure = Some(failure);
    }
}

/// Applies a page only when it belongs to the current listing (BR3.6).
/// Returns `None` when the response is stale and must be dropped.
pub fn apply_page_if_current(
    current: Option<&mut LogGroupListing>,
    listing_id: &str,
    groups: Vec<LogGroup>,
    next_token: Option<&str>,
) -> Option<PageDecision> {
    current
        .filter(|listing| listing.listing_id == listing_id)
        .map(|listing| listing.apply_page(groups, next_token))
}

/// Applies a failure only when it belongs to the current listing (BR3.6).
/// Returns whether it was applied.
pub fn apply_failure_if_current(
    current: Option<&mut LogGroupListing>,
    listing_id: &str,
    failure: ApiFailure,
) -> bool {
    match current.filter(|listing| listing.listing_id == listing_id) {
        Some(listing) => {
            listing.apply_failure(failure);
            true
        }
        None => false,
    }
}

/// Groups whose name contains the trimmed filter, ignoring case; all groups
/// when the filter is blank (BR3.7).
pub fn filter_groups<'a>(groups: &'a [LogGroup], filter: &str) -> Vec<&'a LogGroup> {
    let needle = filter.trim().to_lowercase();
    groups
        .iter()
        .filter(|group| needle.is_empty() || group.log_group_name.to_lowercase().contains(&needle))
        .collect()
}

/// Which empty-list message applies, if any (BR3.10).
pub fn empty_state(listing: &LogGroupListing, filter: &str) -> Option<EmptyState> {
    if listing.groups.is_empty() {
        return (listing.status == ListingStatus::Complete).then_some(EmptyState::NoGroups);
    }
    filter_groups(&listing.groups, filter)
        .is_empty()
        .then_some(EmptyState::NoMatches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure::{FailureKind, SafeDetailFields};

    fn group(name: &str) -> LogGroup {
        LogGroup {
            log_group_name: name.to_string(),
            arn: None,
            creation_time: None,
        }
    }

    fn listing() -> LogGroupListing {
        LogGroupListing::start(
            "listing-1".to_string(),
            ProfileSelector::Named("dev".to_string()),
            "ap-northeast-1".to_string(),
        )
    }

    fn names(groups: &[&LogGroup]) -> Vec<String> {
        groups.iter().map(|g| g.log_group_name.clone()).collect()
    }

    fn failure(kind: FailureKind) -> ApiFailure {
        ApiFailure::new(kind, &SafeDetailFields::default())
    }

    #[test]
    fn starts_loading_and_empty() {
        let listing = listing();
        assert_eq!(listing.status(), ListingStatus::Loading);
        assert!(listing.groups().is_empty());
        assert_eq!(listing.listing_id(), "listing-1");
        assert_eq!(listing.region(), "ap-northeast-1");
        assert_eq!(
            listing.profile(),
            &ProfileSelector::Named("dev".to_string())
        );
        assert_eq!(empty_state(&listing, ""), None, "loading is not empty yet");
    }

    #[test]
    fn page_without_token_completes_the_listing() {
        let mut listing = listing();
        assert_eq!(
            listing.apply_page(vec![group("a")], None),
            PageDecision::Finished
        );
        assert_eq!(listing.status(), ListingStatus::Complete);
    }

    #[test]
    fn same_token_ends_and_a_new_token_continues() {
        let mut listing = listing();
        assert_eq!(
            listing.apply_page(vec![group("a")], Some("t1")),
            PageDecision::Continue
        );
        assert_eq!(listing.status(), ListingStatus::Loading);
        assert_eq!(
            listing.apply_page(vec![group("b")], Some("t2")),
            PageDecision::Continue
        );
        assert_eq!(
            listing.apply_page(vec![], Some("t2")),
            PageDecision::Finished
        );
        assert_eq!(listing.status(), ListingStatus::Complete);
    }

    #[test]
    fn pages_are_merged_in_name_order() {
        let mut listing = listing();
        listing.apply_page(
            vec![group("/b"), group("/aws/lambda/MyFunction")],
            Some("t1"),
        );
        listing.apply_page(vec![group("/a"), group("/c")], None);
        let all: Vec<&LogGroup> = listing.groups().iter().collect();
        assert_eq!(names(&all), ["/a", "/aws/lambda/MyFunction", "/b", "/c"]);
        assert!(listing.contains("/c"));
        assert!(!listing.contains("/d"));
    }

    #[test]
    fn filter_is_trimmed_case_insensitive_substring() {
        let groups = vec![
            group("/aws/lambda/MyFunction"),
            group("/aws/ecs/web"),
            group("app"),
        ];
        assert_eq!(
            names(&filter_groups(&groups, "  myfunc ")),
            ["/aws/lambda/MyFunction"]
        );
        assert_eq!(names(&filter_groups(&groups, "AWS")).len(), 2);
        assert_eq!(names(&filter_groups(&groups, "   ")).len(), 3);
        assert!(filter_groups(&groups, "zzz").is_empty());
    }

    #[test]
    fn empty_states_distinguish_no_groups_from_no_matches() {
        let mut empty = listing();
        empty.apply_page(vec![], None);
        assert_eq!(empty_state(&empty, ""), Some(EmptyState::NoGroups));
        assert_eq!(empty_state(&empty, "x"), Some(EmptyState::NoGroups));

        let mut some = listing();
        some.apply_page(vec![group("alpha")], None);
        assert_eq!(empty_state(&some, "ALP"), None);
        assert_eq!(empty_state(&some, "beta"), Some(EmptyState::NoMatches));
    }

    #[test]
    fn failure_keeps_fetched_groups_as_partial() {
        let mut listing = listing();
        listing.apply_page(vec![group("a")], Some("t1"));
        listing.apply_failure(failure(FailureKind::Throttled));
        assert_eq!(listing.status(), ListingStatus::Partial);
        assert_eq!(listing.groups().len(), 1);
        assert_eq!(
            listing.failure().map(ApiFailure::kind),
            Some(FailureKind::Throttled)
        );

        let mut first_page_failed = self::listing();
        first_page_failed.apply_failure(failure(FailureKind::AccessDenied));
        assert_eq!(first_page_failed.status(), ListingStatus::Partial);
        assert!(first_page_failed.groups().is_empty());
        assert_eq!(
            empty_state(&first_page_failed, ""),
            None,
            "partial shows the error instead"
        );
    }

    #[test]
    fn stale_or_orphan_responses_are_dropped() {
        let mut current = listing();
        assert_eq!(
            apply_page_if_current(Some(&mut current), "old-listing", vec![group("x")], None),
            None
        );
        assert!(current.groups().is_empty());
        assert_eq!(
            apply_page_if_current(None, "listing-1", vec![group("x")], None),
            None
        );
        assert!(!apply_failure_if_current(
            Some(&mut current),
            "old-listing",
            failure(FailureKind::Network)
        ));
        assert!(!apply_failure_if_current(
            None,
            "listing-1",
            failure(FailureKind::Network)
        ));
        assert_eq!(current.status(), ListingStatus::Loading);

        assert_eq!(
            apply_page_if_current(Some(&mut current), "listing-1", vec![group("x")], Some("t")),
            Some(PageDecision::Continue)
        );
        assert!(apply_failure_if_current(
            Some(&mut current),
            "listing-1",
            failure(FailureKind::Network)
        ));
        assert_eq!(current.status(), ListingStatus::Partial);
    }

    #[test]
    fn arn_is_not_sent_to_the_screen() {
        let mut g = group("a");
        g.arn = Some("arn:aws:logs:ap-northeast-1:123456789012:log-group:a".to_string());
        let json = serde_json::to_string(&g).unwrap();
        assert!(!json.contains("123456789012"));
        assert!(json.contains("\"logGroupName\":\"a\""));
    }
}
