//! Kernel events emitted for audit, replay, and future UI projections.

use crate::{ArtifactId, ProjectId, RunId, RunState, StepId, ThreadId};
use serde::{Deserialize, Serialize};

/// Monotonic event ordering value assigned by the future ledger boundary.
///
/// The sequence is deterministic by construction: replay starts at zero and
/// advances by one for each accepted event. Wall-clock timestamps deliberately
/// stay out of the core event model.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EventSequence(u64);

impl EventSequence {
    /// Creates a sequence from an existing deterministic ledger value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the first sequence value used by a fresh event stream.
    pub const fn initial() -> Self {
        Self(0)
    }

    /// Returns the numeric value for storage adapters and tests.
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Returns the next sequence value, or `None` if the counter is exhausted.
    pub fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Ordered event record used by replay, audit, and future projections.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CoreEventEnvelope {
    sequence: EventSequence,
    event: CoreEvent,
}

impl CoreEventEnvelope {
    /// Wraps an event payload with its deterministic sequence number.
    pub fn new(sequence: EventSequence, event: CoreEvent) -> Self {
        Self { sequence, event }
    }

    /// Returns the deterministic event sequence.
    pub fn sequence(&self) -> EventSequence {
        self.sequence
    }

    /// Returns the enclosed core event payload.
    pub fn event(&self) -> &CoreEvent {
        &self.event
    }
}

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
    /// A run was created inside a thread.
    RunCreated {
        /// Owning thread identifier.
        thread_id: ThreadId,
        /// Created run identifier.
        run_id: RunId,
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

impl CoreEvent {
    /// Creates a run lifecycle transition event.
    pub fn run_state_changed(run_id: RunId, from: RunState, to: RunState) -> Self {
        Self::RunStateChanged { run_id, from, to }
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreEvent, CoreEventEnvelope, EventSequence};
    use crate::{RunId, RunState};
    use uuid::Uuid;

    fn fixed_run_id() -> RunId {
        RunId::from_uuid(Uuid::from_u128(1))
    }

    #[test]
    fn event_sequence_starts_deterministically() {
        assert_eq!(EventSequence::initial().as_u64(), 0);
    }

    #[test]
    fn event_sequence_advances_deterministically() {
        let initial = EventSequence::initial();
        let next = initial.checked_next();

        assert_eq!(next, Some(EventSequence::new(1)));
    }

    #[test]
    fn event_envelope_can_wrap_run_state_change() {
        let event = CoreEvent::RunStateChanged {
            run_id: fixed_run_id(),
            from: RunState::Created,
            to: RunState::Planning,
        };

        let envelope = CoreEventEnvelope::new(EventSequence::initial(), event.clone());

        assert_eq!(envelope.sequence(), EventSequence::initial());
        assert_eq!(envelope.event(), &event);
    }

    #[test]
    fn run_state_change_event_preserves_transition_fields() {
        let run_id = fixed_run_id();
        let event = CoreEvent::RunStateChanged {
            run_id,
            from: RunState::Created,
            to: RunState::Planning,
        };

        assert_eq!(
            event,
            CoreEvent::RunStateChanged {
                run_id,
                from: RunState::Created,
                to: RunState::Planning,
            }
        );
    }
}
