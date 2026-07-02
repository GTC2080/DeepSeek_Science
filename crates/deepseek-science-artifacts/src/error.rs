//! Error types for artifact metadata.

use thiserror::Error;

/// Errors raised while constructing or validating artifact metadata.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ArtifactError {
    /// Artifact content hash was empty.
    #[error("artifact content hash must not be empty")]
    EmptyContentHash,
}
