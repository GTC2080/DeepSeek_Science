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

/// Reason an atomic write request cannot be planned.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum WriteRequestViolation {
    /// The validated target path does not end in a file name.
    #[error("target path must include a file name")]
    MissingTargetFileName,
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
    /// An atomic write request is invalid before filesystem access occurs.
    #[error("invalid write request: {reason}")]
    InvalidWriteRequest {
        /// Specific request validation failure.
        reason: WriteRequestViolation,
    },
    /// A future write target's parent directory does not exist.
    #[error("parent directory is missing for {path:?}")]
    ParentDirectoryMissing {
        /// Final target path whose parent is missing.
        path: PathBuf,
    },
    /// Create-new mode found an existing target.
    #[error("target already exists: {path:?}")]
    TargetAlreadyExists {
        /// Existing final target path.
        path: PathBuf,
    },
    /// Replace-existing mode did not find an existing target.
    #[error("target is missing: {path:?}")]
    TargetMissing {
        /// Missing final target path.
        path: PathBuf,
    },
    /// Writing the temporary sibling failed.
    #[error("write failed for {path:?}: {reason}")]
    WriteFailed {
        /// Temporary path that could not be written.
        path: PathBuf,
        /// Failure reason with sensitive data already removed.
        reason: String,
    },
    /// Renaming a temporary sibling into the final target failed.
    #[error("rename failed from {from:?} to {to:?}: {reason}")]
    RenameFailed {
        /// Temporary path that was written first.
        from: PathBuf,
        /// Final target path.
        to: PathBuf,
        /// Failure reason with sensitive data already removed.
        reason: String,
    },
    /// A storage backend failed with a human-readable reason.
    #[error("storage backend failed: {reason}")]
    Backend {
        /// Failure reason with sensitive data already removed.
        reason: String,
    },
}
