//! Error type for storage interfaces.

use thiserror::Error;

/// Errors surfaced by future storage implementations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// A storage backend failed with a human-readable reason.
    #[error("storage backend failed: {reason}")]
    Backend {
        /// Failure reason with sensitive data already removed.
        reason: String,
    },
}
