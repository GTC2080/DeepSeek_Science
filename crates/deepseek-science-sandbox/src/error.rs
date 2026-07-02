//! Error type for sandbox policy and execution boundaries.

use crate::ExecutionPermission;
use thiserror::Error;

/// Errors produced by sandbox policy checks or future runners.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum SandboxError {
    /// A request needs approval before it can proceed.
    #[error("sandbox approval required: {reason}")]
    ApprovalRequired {
        /// Permissions that require approval before execution.
        permissions: Vec<ExecutionPermission>,
        /// Approval reason safe to show in logs.
        reason: String,
    },
    /// A requested permission was denied by policy.
    #[error("sandbox permission denied: {reason}")]
    PermissionDenied {
        /// Permissions denied by policy.
        permissions: Vec<ExecutionPermission>,
        /// Denial reason safe to show in logs.
        reason: String,
    },
    /// A request asks for a permission that cannot be approved.
    #[error("sandbox request forbidden: {reason}")]
    ForbiddenRequest {
        /// Permissions forbidden by policy.
        permissions: Vec<ExecutionPermission>,
        /// Rejection reason safe to show in logs.
        reason: String,
    },
    /// A request is malformed before policy evaluation.
    #[error("invalid sandbox request: {reason}")]
    InvalidRequest {
        /// Validation reason safe to show in logs.
        reason: String,
    },
    /// Execution is intentionally not implemented by this sandbox layer.
    #[error("sandbox execution unsupported: {reason}")]
    UnsupportedExecution {
        /// Explanation safe to show in logs.
        reason: String,
    },
}
