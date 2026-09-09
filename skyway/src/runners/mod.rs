//! Runners execute a [`crate::plan::PipelinePlan`].
//!
//! A runner decides how work is scheduled: thread counts, channels, and where
//! sequence restoration happens. It does not revisit the plan's decisions.

mod parallel;
pub use parallel::{ParallelOptions, PipelineOutput, PipelineWorker, run_pipeline};

#[cfg(feature = "filter")]
mod spool;
