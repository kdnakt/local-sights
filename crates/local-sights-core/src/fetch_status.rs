//! FetchStatus: the status of one fetch job (261012-parallel-fetch
//! U1:BR1.15, BR1.17, contract C3).
//!
//! [`JobStatus`] moved here from the coordinator in U1 with its values and
//! serde names unchanged, so the cache plan, the session, the desktop app,
//! the `fetch_check` example and the tests can depend on the status without
//! depending on the coordinator. Deciding the status stays in the
//! coordinator (`decide_job_status`, U1:BR1.15).

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
    /// Stopped by the caller (BR5.5).
    Aborted,
}

impl JobStatus {
    /// Every status, in declaration order.
    pub const ALL: [JobStatus; 5] = [
        JobStatus::Running,
        JobStatus::Completed,
        JobStatus::Failed,
        JobStatus::CompletedWithFailures,
        JobStatus::Aborted,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_values_are_the_same_five_as_before_the_move() {
        assert_eq!(
            JobStatus::ALL.map(JobStatus::as_str),
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
