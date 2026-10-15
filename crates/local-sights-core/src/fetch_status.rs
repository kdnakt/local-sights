//! FetchStatus: the status of one fetch job (261012-parallel-fetch
//! U1:BR1.15, BR1.17, contract C3).
//!
//! [`JobStatus`] moved here from the coordinator in U1 with its values and
//! serde names unchanged, so the cache plan, the session, the desktop app,
//! the `fetch_check` example and the tests can depend on the status without
//! depending on the coordinator. Deciding the status stays in the
//! coordinator (`decide_job_status`, U1:BR1.15).
//!
//! U5 (u5-cancel) adds [`JobStatus::Cancelled`] as the sixth value, last, so
//! the five earlier values and their serde names stay as they were.

use serde::Serialize;

/// Status of a fetch job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum JobStatus {
    /// In progress.
    Running,
    /// Every selected stream was read (also when none was selected).
    Completed,
    /// The listing failed on its first page and nothing was selected
    /// (including RegionMissing).
    Failed,
    /// Finished, but some streams failed or the listing was partial.
    CompletedWithFailures,
    /// Stopped because the window closes; everything fetched is discarded
    /// (BR5.5, U1:BR1.16).
    Aborted,
    /// Stopped by [Cancel] before every stream was read (U5:BR5.5): the
    /// fetched logs stay and nothing is written to the cache (BR5.7).
    Cancelled,
}

impl JobStatus {
    /// Every status, in declaration order.
    pub const ALL: [JobStatus; 6] = [
        JobStatus::Running,
        JobStatus::Completed,
        JobStatus::Failed,
        JobStatus::CompletedWithFailures,
        JobStatus::Aborted,
        JobStatus::Cancelled,
    ];

    /// The fixed word of the status, the same as its serde name. Used by
    /// the one-line fetch summary (NFR8.6), which takes no free text.
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Running => "Running",
            JobStatus::Completed => "Completed",
            JobStatus::Failed => "Failed",
            JobStatus::CompletedWithFailures => "CompletedWithFailures",
            JobStatus::Aborted => "Aborted",
            JobStatus::Cancelled => "Cancelled",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_values_are_the_same_five_as_before_the_move() {
        assert_eq!(
            JobStatus::ALL[..5]
                .iter()
                .map(|status| status.as_str())
                .collect::<Vec<_>>(),
            [
                "Running",
                "Completed",
                "Failed",
                "CompletedWithFailures",
                "Aborted"
            ]
        );
    }

    #[test]
    fn cancelled_is_the_sixth_value_with_its_own_word() {
        // U5 (u5-cancel) BR5.5, NFR8.13: one fixed word, added last.
        assert_eq!(JobStatus::ALL.len(), 6);
        assert_eq!(JobStatus::ALL[5], JobStatus::Cancelled);
        assert_eq!(JobStatus::Cancelled.as_str(), "Cancelled");
        assert_eq!(
            serde_json::to_string(&JobStatus::Cancelled).unwrap(),
            "\"Cancelled\""
        );
    }

    #[test]
    fn the_serde_names_are_unchanged_and_match_the_words() {
        for status in JobStatus::ALL {
            assert_eq!(
                serde_json::to_string(&status).unwrap(),
                format!("\"{}\"", status.as_str())
            );
        }
        assert_eq!(
            serde_json::to_string(&JobStatus::CompletedWithFailures).unwrap(),
            "\"CompletedWithFailures\""
        );
    }
}
