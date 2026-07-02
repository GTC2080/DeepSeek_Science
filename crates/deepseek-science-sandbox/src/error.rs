//! Error type for sandbox policy and execution boundaries.

use thiserror::Error;

/// Errors produced by sandbox policy checks or future runners.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum SandboxError {
    /// A requested permission was denied.
    #[error("sandbox permission denied: {reason}")]
    PermissionDenied {
        /// Denial reason safe to show in logs.
        reason: String,
    },
}
