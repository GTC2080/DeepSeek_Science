#![forbid(unsafe_code)]
//! Storage interfaces and deterministic project layout helpers.
//!
//! Phase 1 defines repository traits only. SQLite or other durable backends can
//! be added later behind explicit implementations and tests.

pub mod error;
pub mod layout;
pub mod repository;

pub use error::{PathSafetyViolation, StorageError};
pub use layout::{ProjectLayout, StorageLayout, StorageRoot};
pub use repository::{ArtifactRepository, ProjectRepository, RunRepository};
