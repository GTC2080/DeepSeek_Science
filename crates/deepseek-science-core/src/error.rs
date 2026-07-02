//! Error types for the domain-neutral kernel.

use crate::RunState;
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
}
