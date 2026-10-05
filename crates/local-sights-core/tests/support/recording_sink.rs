//! A [`FetchSink`] that records everything it receives, in order.

use local_sights_core::coordinator::{BatchProgress, FetchJob, FetchSink, JobStatus};
use local_sights_core::fetcher::{StreamFetchOutcome, StreamStatus};
use local_sights_core::streams::planner::ListingProgress;

/// One delivery to the sink.
#[derive(Debug, Clone, PartialEq)]
pub enum Delivered {
    /// The fetch started.
    Started { job_id: u64, timeline_version: u64 },
    /// One listing page was read.
    Listing { seen: u32, selected: u32 },
    /// The streams to fetch were decided.
    Planned { planned: u32 },
    /// One page was added to the timeline.
    Batch {
        added: u64,
        total: u64,
        timeline_version: u64,
    },
    /// One stream finished.
    StreamFinished {
        name: String,
        status: StreamStatus,
        finished: u32,
    },
    /// The fetch finished.
    Finished(Box<FetchJob>),
}

/// Records every delivery.
#[derive(Debug, Default)]
pub struct RecordingSink {
    pub delivered: Vec<Delivered>,
    /// The timeline version given with `on_finished`.
    pub finished_timeline_version: Option<u64>,
}

impl RecordingSink {
    /// The deliveries without their details, e.g. `["Started", "Listing"]`.
    pub fn kinds(&self) -> Vec<&'static str> {
        self.delivered
            .iter()
            .map(|delivered| match delivered {
                Delivered::Started { .. } => "Started",
                Delivered::Listing { .. } => "Listing",
                Delivered::Planned { .. } => "Planned",
                Delivered::Batch { .. } => "Batch",
                Delivered::StreamFinished { .. } => "StreamFinished",
                Delivered::Finished(_) => "Finished",
            })
            .collect()
    }
}

impl FetchSink for RecordingSink {
    fn on_started(&mut self, job: &FetchJob, timeline_version: u64) {
        assert_eq!(job.status, JobStatus::Running);
        self.delivered.push(Delivered::Started {
            job_id: job.job_id,
            timeline_version,
        });
    }

    fn on_listing_progress(&mut self, _job_id: u64, progress: ListingProgress) {
        self.delivered.push(Delivered::Listing {
            seen: progress.seen_stream_count,
            selected: progress.selected_stream_count,
        });
    }

    fn on_planned(&mut self, job: &FetchJob) {
        self.delivered.push(Delivered::Planned {
            planned: job.planned_stream_count,
        });
    }

    fn on_batch(&mut self, _job_id: u64, progress: BatchProgress) {
        self.delivered.push(Delivered::Batch {
            added: progress.added,
            total: progress.total,
            timeline_version: progress.timeline_version,
        });
    }

    fn on_stream_finished(&mut self, job: &FetchJob, outcome: &StreamFetchOutcome) {
        assert!(job.finished_stream_count <= job.planned_stream_count);
        self.delivered.push(Delivered::StreamFinished {
            name: outcome.log_stream_name.clone(),
            status: outcome.status,
            finished: job.finished_stream_count,
        });
    }

    fn on_finished(&mut self, job: &FetchJob, timeline_version: u64) {
        self.finished_timeline_version = Some(timeline_version);
        self.delivered
            .push(Delivered::Finished(Box::new(job.clone())));
    }
}
