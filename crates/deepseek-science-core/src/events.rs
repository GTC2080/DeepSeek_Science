//! Kernel events emitted for audit, replay, and future UI projections.

use crate::{ArtifactId, ProjectId, RunId, RunState, StepId, ThreadId};
use serde::{Deserialize, Serialize};

/// Domain-neutral event emitted by the core runtime.
///
/// Events are intentionally compact. Domain packs can add their own event logs
/// later without requiring chemistry or other domain concepts in the kernel.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CoreEvent {
    /// A project was created.
    ProjectCreated {
        /// Created project identifier.
        project_id: ProjectId,
    },
    /// A thread was created inside a project.
    ThreadCreated {
        /// Owning project identifier.
        project_id: ProjectId,
        /// Created thread identifier.
        thread_id: ThreadId,
    },
    /// A run moved between lifecycle states.
    RunStateChanged {
        /// Run that changed state.
        run_id: RunId,
        /// Previous state.
        from: RunState,
        /// New state.
        to: RunState,
    },
    /// A step was recorded for a run.
    StepRecorded {
        /// Owning run identifier.
        run_id: RunId,
        /// Recorded step identifier.
        step_id: StepId,
    },
    /// An artifact was recorded in the audit ledger.
    ArtifactRecorded {
        /// Recorded artifact identifier.
        artifact_id: ArtifactId,
    },
}
