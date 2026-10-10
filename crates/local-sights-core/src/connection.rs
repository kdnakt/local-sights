//! Connection selection state machine (AppSession, U2).
//!
//! Tracks the chosen profile and region (BR2.1, BR2.2), decides whether a
//! change needs confirmation because logs are shown (BR2.5), and tells the
//! caller to start a new log group listing or to clear it whenever a change
//! is applied (BR2.3, BR3.6). Pure logic: no I/O.

use serde::Serialize;

use crate::catalog::ProfileSelector;

/// The chosen connection. Both parts may be unselected.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSelection {
    /// Chosen profile.
    pub profile: Option<ProfileSelector>,
    /// Chosen region code.
    pub region: Option<String>,
}

/// A change waiting for the user's confirmation. Holds at least one part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingConnectionChange {
    proposed_profile: Option<ProfileSelector>,
    proposed_region: Option<String>,
}

/// What to do with the log group listing after an applied change. Either
/// way, the previous listing is no longer current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListingCommand {
    /// The connection is complete: start a listing with this new ID.
    Start {
        /// ID of the new listing.
        listing_id: String,
    },
    /// The connection is incomplete: show no list.
    Clear,
}

/// Result of asking to change the connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeOutcome {
    /// Same as the current choice; nothing happens.
    Unchanged,
    /// Logs are shown: the change waits for confirmation.
    AwaitingConfirmation,
    /// Another change is already waiting for confirmation.
    Blocked,
    /// The change was applied.
    Applied(ListingCommand),
}

/// Selection plus the change awaiting confirmation, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionState {
    selection: ConnectionSelection,
    pending: Option<PendingConnectionChange>,
}

impl ConnectionSelection {
    /// Whether both profile and region are chosen.
    pub fn is_complete(&self) -> bool {
        self.profile.is_some() && self.region.is_some()
    }
}

impl PendingConnectionChange {
    /// The proposed profile, when the profile changes.
    pub fn proposed_profile(&self) -> Option<&ProfileSelector> {
        self.proposed_profile.as_ref()
    }

    /// The proposed region, when only the region changes.
    pub fn proposed_region(&self) -> Option<&str> {
        self.proposed_region.as_deref()
    }
}

impl ConnectionState {
    /// Nothing selected (BR2.1).
    pub fn new() -> Self {
        Self::default()
    }

    /// The current selection.
    pub fn selection(&self) -> &ConnectionSelection {
        &self.selection
    }

    /// The change awaiting confirmation.
    pub fn pending(&self) -> Option<&PendingConnectionChange> {
        self.pending.as_ref()
    }

    /// Asks to choose a profile; `default_region` is that profile's default
    /// region, used as the region when the change is applied (BR2.2).
    pub fn request_profile(
        &mut self,
        profile: ProfileSelector,
        default_region: Option<String>,
        has_logs: bool,
    ) -> ChangeOutcome {
        if self.pending.is_some() {
            return ChangeOutcome::Blocked;
        }
        if self.selection.profile.as_ref() == Some(&profile) {
            return ChangeOutcome::Unchanged;
        }
        if has_logs {
            self.pending = Some(PendingConnectionChange {
                proposed_profile: Some(profile),
                proposed_region: None,
            });
            return ChangeOutcome::AwaitingConfirmation;
        }
        ChangeOutcome::Applied(self.apply(Some(profile), default_region))
    }

    /// Asks to choose a region.
    pub fn request_region(&mut self, region: String, has_logs: bool) -> ChangeOutcome {
        if self.pending.is_some() {
            return ChangeOutcome::Blocked;
        }
        if self.selection.region.as_ref() == Some(&region) {
            return ChangeOutcome::Unchanged;
        }
        if has_logs {
            self.pending = Some(PendingConnectionChange {
                proposed_profile: None,
                proposed_region: Some(region),
            });
            return ChangeOutcome::AwaitingConfirmation;
        }
        let profile = self.selection.profile.clone();
        ChangeOutcome::Applied(self.apply(profile, Some(region)))
    }

    /// Applies the pending change; `default_region_of` gives a profile's
    /// default region (BR2.2). `None` when nothing was pending.
    pub fn confirm(
        &mut self,
        default_region_of: impl Fn(&ProfileSelector) -> Option<String>,
    ) -> Option<ListingCommand> {
        let pending = self.pending.take()?;
        let command = match (pending.proposed_profile, pending.proposed_region) {
            (Some(profile), _) => {
                let region = default_region_of(&profile);
                self.apply(Some(profile), region)
            }
            (None, region) => {
                let profile = self.selection.profile.clone();
                let region = region.or_else(|| self.selection.region.clone());
                self.apply(profile, region)
            }
        };
        Some(command)
    }

    /// Drops the pending change and keeps the current selection. Returns
    /// whether something was pending.
    pub fn cancel(&mut self) -> bool {
        self.pending.take().is_some()
    }

    /// A new listing for the current connection (reload, BR3.5), or `None`
    /// when the connection is incomplete.
    pub fn reload(&self) -> Option<ListingCommand> {
        self.selection.is_complete().then(new_listing)
    }

    /// Sets the selection and decides what happens to the listing.
    fn apply(
        &mut self,
        profile: Option<ProfileSelector>,
        region: Option<String>,
    ) -> ListingCommand {
        self.selection = ConnectionSelection { profile, region };
        if self.selection.is_complete() {
            new_listing()
        } else {
            ListingCommand::Clear
        }
    }
}

/// A listing command with a fresh ID.
fn new_listing() -> ListingCommand {
    ListingCommand::Start {
        listing_id: uuid::Uuid::new_v4().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev() -> ProfileSelector {
        ProfileSelector::Named("dev".to_string())
    }

    fn prod() -> ProfileSelector {
        ProfileSelector::Named("prod".to_string())
    }

    fn started(outcome: &ChangeOutcome) -> Option<String> {
        match outcome {
            ChangeOutcome::Applied(ListingCommand::Start { listing_id }) => {
                Some(listing_id.clone())
            }
            _ => None,
        }
    }

    fn connected() -> ConnectionState {
        let mut state = ConnectionState::new();
        state.request_profile(dev(), Some("ap-northeast-1".to_string()), false);
        state
    }

    #[test]
    fn starts_with_nothing_selected_and_no_listing() {
        let state = ConnectionState::new();
        assert_eq!(state.selection(), &ConnectionSelection::default());
        assert!(!state.selection().is_complete());
        assert!(state.pending().is_none());
        assert_eq!(state.reload(), None);
    }

    #[test]
    fn profile_with_default_region_connects_and_starts_a_listing() {
        let mut state = ConnectionState::new();
        let outcome = state.request_profile(dev(), Some("ap-northeast-1".to_string()), false);
        assert!(started(&outcome).is_some());
        assert_eq!(state.selection().profile, Some(dev()));
        assert_eq!(state.selection().region.as_deref(), Some("ap-northeast-1"));
        assert!(state.selection().is_complete());
    }

    #[test]
    fn profile_without_default_region_leaves_region_unselected() {
        let mut state = ConnectionState::new();
        let outcome = state.request_profile(ProfileSelector::SdkDefault, None, false);
        assert_eq!(outcome, ChangeOutcome::Applied(ListingCommand::Clear));
        assert_eq!(state.selection().region, None);
        let outcome = state.request_region("us-east-1".to_string(), false);
        assert!(started(&outcome).is_some());
        assert!(state.selection().is_complete());
    }

    #[test]
    fn without_logs_changes_apply_at_once_with_a_new_listing() {
        let mut state = connected();
        let first = state.reload().unwrap();
        let outcome = state.request_region("eu-west-1".to_string(), false);
        let second = started(&outcome).unwrap();
        assert_ne!(
            ListingCommand::Start {
                listing_id: second.clone()
            },
            first
        );
        assert_eq!(state.selection().region.as_deref(), Some("eu-west-1"));
        let outcome = state.request_profile(prod(), None, false);
        assert_eq!(outcome, ChangeOutcome::Applied(ListingCommand::Clear));
        assert_eq!(state.selection().profile, Some(prod()));
        assert_eq!(state.selection().region, None);
    }

    #[test]
    fn choosing_the_current_value_changes_nothing() {
        let mut state = connected();
        assert_eq!(
            state.request_profile(dev(), Some("eu-west-1".to_string()), true),
            ChangeOutcome::Unchanged
        );
        assert_eq!(
            state.request_region("ap-northeast-1".to_string(), true),
            ChangeOutcome::Unchanged
        );
        assert!(state.pending().is_none());
    }

    #[test]
    fn with_logs_a_change_waits_and_confirm_applies_it() {
        let mut state = connected();
        assert_eq!(
            state.request_profile(prod(), Some("us-east-1".to_string()), true),
            ChangeOutcome::AwaitingConfirmation
        );
        let pending = state.pending().unwrap();
        assert_eq!(pending.proposed_profile(), Some(&prod()));
        assert_eq!(pending.proposed_region(), None);
        assert_eq!(state.selection().profile, Some(dev()), "not applied yet");
        assert_eq!(
            state.request_region("eu-west-1".to_string(), true),
            ChangeOutcome::Blocked
        );

        let command =
            state.confirm(|profile| (profile == &prod()).then(|| "us-west-2".to_string()));
        assert!(matches!(command, Some(ListingCommand::Start { .. })));
        assert_eq!(state.selection().profile, Some(prod()));
        assert_eq!(state.selection().region.as_deref(), Some("us-west-2"));
        assert!(state.pending().is_none());
        assert_eq!(state.confirm(|_| None), None);
    }

    #[test]
    fn cancel_keeps_the_previous_selection() {
        let mut state = connected();
        assert_eq!(
            state.request_region("eu-west-1".to_string(), true),
            ChangeOutcome::AwaitingConfirmation
        );
        assert_eq!(
            state.pending().unwrap().proposed_region(),
            Some("eu-west-1")
        );
        assert_eq!(state.pending().unwrap().proposed_profile(), None);
        assert!(state.cancel());
        assert!(state.pending().is_none());
        assert_eq!(state.selection().region.as_deref(), Some("ap-northeast-1"));
        assert!(!state.cancel());
    }

    #[test]
    fn confirming_a_profile_without_default_region_clears_the_listing() {
        let mut state = connected();
        state.request_profile(prod(), None, true);
        assert_eq!(state.confirm(|_| None), Some(ListingCommand::Clear));
        assert_eq!(state.selection().region, None);
        assert_eq!(state.reload(), None);
    }
}
