//! Error type for storage interfaces.

use std::path::PathBuf;
use thiserror::Error;

/// Reason a caller-supplied logical path cannot be joined under a storage root.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum PathSafetyViolation {
    /// Empty paths are ambiguous and do not name a storage object.
    #[error("path is empty")]
    Empty,
    /// Absolute paths would bypass the storage root.
    #[error("absolute path is not allowed")]
    Absolute,
    /// Platform prefix components, such as Windows drive prefixes, are not allowed.
    #[error("path prefix component is not allowed")]
    Prefix,
    /// Root directory components would escape the logical relative layout.
    #[error("root directory component is not allowed")]
    Root,
    /// Parent components would allow path traversal.
    #[error("parent directory component is not allowed")]
    Parent,
    /// Current-directory components make logical paths less deterministic.
    #[error("current directory component is not allowed")]
    CurrentDir,
}

/// Errors surfaced by future storage implementations.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum StorageError {
    /// The storage root path is invalid before any filesystem access occurs.
    #[error("invalid storage root: {reason}")]
    InvalidStorageRoot {
        /// Human-readable validation reason.
        reason: String,
    },
    /// A logical relative path failed path-safety validation.
    #[error("unsafe relative path {path:?}: {reason}")]
    UnsafeRelativePath {
        /// Caller-supplied path that failed validation.
        path: PathBuf,
        /// Specific safety violation.
        reason: PathSafetyViolation,
    },
    /// A storage backend failed with a human-readable reason.
    #[error("storage backend failed: {reason}")]
    Backend {
        /// Failure reason with sensitive data already removed.
        reason: String,
    },
}
