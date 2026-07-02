//! Error types for the domain-neutral kernel.

use crate::{EventSequence, RunId, RunState};
use thiserror::Error;

/// Errors raised by core entity constructors and state transitions.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CoreError {
    /// A user-facing label was empty after whitespace trimming.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Name of the rejected field.
        field: &'static str,
    },

    /// A run was asked to move through a transition the kernel does not allow.
    #[error("invalid run state transition from {from} to {to}")]
    InvalidRunTransition {
        /// Current run state.
        from: RunState,
        /// Requested run state.
        to: RunState,
    },

    /// A replay event did not have the expected deterministic sequence.
    #[error("event sequence out of order: expected {expected:?}, found {found:?}")]
    EventSequenceOutOfOrder {
        /// Sequence required at this position in the stream.
        expected: EventSequence,
        /// Sequence found in the input stream.
        found: EventSequence,
    },

    /// A run inspection received an event for a different run.
    #[error("event run id mismatch: expected {expected}, found {found}")]
    EventRunIdMismatch {
        /// Run requested by the projection caller.
        expected: RunId,
        /// Run carried by the rejected event.
        found: RunId,
    },

    /// A run inspection stream did not start with the required run creation event.
    #[error("missing RunCreated event for run {run_id}")]
    MissingRunCreated {
        /// Run requested by the projection caller.
        run_id: RunId,
    },

    /// A replayed run state event is inconsistent with the projected lifecycle.
    #[error(
        "invalid replay transition while current state is {current}: event claims {event_from} to {event_to}"
    )]
    InvalidReplayTransition {
        /// Current projected state before applying the event.
        current: RunState,
        /// Previous state recorded by the event.
        event_from: RunState,
        /// Next state recorded by the event.
        event_to: RunState,
    },

    /// A run inspection received a core event that is not part of a run stream.
    #[error("unexpected {event_kind} event while inspecting run {run_id}")]
    UnexpectedRunInspectionEvent {
        /// Run requested by the projection caller.
        run_id: RunId,
        /// Rejected core event kind.
        event_kind: &'static str,
    },
}
