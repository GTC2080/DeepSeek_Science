//! Artifact kind labels.

use serde::{Deserialize, Serialize};

/// Coarse artifact type used by manifests and review tools.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// UTF-8 markdown document.
    Markdown,
    /// UTF-8 text file.
    Text,
    /// JSON data.
    Json,
    /// CSV table.
    Csv,
    /// Arbitrary binary payload.
    Binary,
}
