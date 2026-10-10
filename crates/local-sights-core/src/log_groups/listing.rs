//! Listing flow: reads every `DescribeLogGroups` page of one connection
//! (BR3.1) and hands each page to a [`ListingSink`] (BR3.2). An error stops
//! the listing as Partial (BR3.4, BR4.1). When the listing is no longer the
//! current one, the flow stops without calling the next page (BR3.6).

use crate::catalog::ProfileSelector;
use crate::failure::ApiFailure;
use crate::gateway::{CloudWatchLogsGateway, DescribeLogGroupsRequest};
use crate::paging::PageDecision;

use super::LogGroup;

/// What to list: the listing ID and the connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingRequest {
    /// ID of the listing (new for every start or reload).
    pub listing_id: String,
    /// Profile of the connection.
    pub profile: ProfileSelector,
    /// Region of the connection.
    pub region: String,
}

/// How a listing run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingEnd {
    /// Every page was read.
    Complete,
    /// An error stopped the listing.
    Partial,
    /// The listing stopped being current; the rest was not read.
    Superseded,
}

/// Receives listing progress, typically by updating the session's current
/// listing with [`super::apply_page_if_current`].
pub trait ListingSink {
    /// Whether `listing_id` is still the current listing.
    fn is_current(&self, listing_id: &str) -> bool;
    /// One page arrived. Returns the paging decision, or `None` when the
    /// page was dropped because the listing is no longer current.
    fn on_page(
        &mut self,
        listing_id: &str,
        groups: Vec<LogGroup>,
        next_token: Option<&str>,
    ) -> Option<PageDecision>;
    /// The listing failed. Returns whether the failure was applied.
    fn on_failure(&mut self, listing_id: &str, failure: ApiFailure) -> bool;
}

/// Runs one listing to its end.
pub async fn run_listing<G, S>(gateway: &G, request: &ListingRequest, sink: &mut S) -> ListingEnd
where
    G: CloudWatchLogsGateway,
    S: ListingSink + Send,
{
    let mut next_token: Option<String> = None;
    loop {
        if !sink.is_current(&request.listing_id) {
            return ListingEnd::Superseded;
        }
        let call = DescribeLogGroupsRequest {
            profile_name: request.profile.profile_name().map(str::to_string),
            region: Some(request.region.clone()),
            next_token: next_token.clone(),
        };
        let page = match gateway.describe_log_groups(call).await {
            Ok(page) => page,
            Err(failure) => {
                return if sink.on_failure(&request.listing_id, failure) {
                    ListingEnd::Partial
                } else {
                    ListingEnd::Superseded
                };
            }
        };
        match sink.on_page(&request.listing_id, page.groups, page.next_token.as_deref()) {
            None => return ListingEnd::Superseded,
            Some(PageDecision::Finished) => return ListingEnd::Complete,
            Some(PageDecision::Continue) => next_token = page.next_token,
        }
    }
}
