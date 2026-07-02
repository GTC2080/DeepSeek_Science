//! Error types for artifact metadata.

use thiserror::Error;

/// Errors raised while constructing or validating artifact metadata.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ArtifactError {
    /// Artifact path was empty.
    #[error("artifact path must not be empty")]
    EmptyPath,
}
