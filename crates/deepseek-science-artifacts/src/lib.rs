#![forbid(unsafe_code)]
//! Artifact manifests and provenance records.
//!
//! Artifacts are the audit boundary between agent runs, generated files, and
//! review status. This crate owns in-memory artifact contracts; it does not
//! write files.

pub mod envelope;
pub mod error;
pub mod hash;
pub mod kind;
pub mod manifest;
pub mod review;

pub use envelope::{
    ArtifactContentDescriptor, ArtifactInputDescriptor, ArtifactProvenance, ArtifactReviewSummary,
    UnregisteredArtifactEnvelope, UnregisteredArtifactMetadata,
};
pub use error::ArtifactError;
pub use hash::{hash_bytes, ExactByteHash, ExactHashAlgorithm};
pub use kind::ArtifactKind;
pub use manifest::{ArtifactManifest, ArtifactRef, ProvenanceRecord};
pub use review::ReviewStatus;
