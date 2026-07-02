//! Domain-neutral artifact kind labels.

use serde::{Deserialize, Serialize};

/// Coarse artifact type used by manifests, storage, and review tools.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// Structured rows or columns such as CSV-derived or computed tables.
    Table,
    /// Visual output such as a plot, chart, diagram, or image.
    Figure,
    /// Human-readable analytical report.
    Report,
    /// Source code or executable text artifact.
    Code,
    /// JSON data.
    Json,
    /// UTF-8 text file.
    Text,
    /// Runtime or diagnostic log.
    Log,
    /// Artifact kind is not known yet.
    Unknown,
}
