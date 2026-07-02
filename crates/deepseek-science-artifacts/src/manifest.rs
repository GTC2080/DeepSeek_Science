//! Artifact manifest and provenance records.

use crate::{hash_bytes, ArtifactError, ArtifactKind, ReviewStatus};
use deepseek_science_core::{ArtifactId, StepId};
use serde::{Deserialize, Serialize};

/// Stable reference to an artifact by identifier and content hash.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Artifact identifier.
    pub id: ArtifactId,
    /// BLAKE3 content hash.
    pub content_hash: String,
}

/// One provenance note linking an artifact to a run step or review action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// Optional run step that produced or reviewed the artifact.
    pub step_id: Option<StepId>,
    /// Short provenance note.
    pub note: String,
}

impl ProvenanceRecord {
    /// Creates a provenance note.
    pub fn new(step_id: Option<StepId>, note: impl Into<String>) -> Self {
        Self {
            step_id,
            note: note.into(),
        }
    }
}

/// Metadata for one generated or imported artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactManifest {
    /// Artifact identifier.
    pub id: ArtifactId,
    /// Coarse artifact kind.
    pub kind: ArtifactKind,
    /// Project-relative artifact path.
    pub path: String,
    /// BLAKE3 hash of artifact content.
    pub content_hash: String,
    /// Provenance chain for audit and replay.
    pub provenance: Vec<ProvenanceRecord>,
    /// Current review status.
    pub review_status: ReviewStatus,
}

impl ArtifactManifest {
    /// Creates a manifest and hashes the supplied bytes.
    pub fn new(
        kind: ArtifactKind,
        path: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, ArtifactError> {
        let path = path.into();
        if path.trim().is_empty() {
            return Err(ArtifactError::EmptyPath);
        }

        Ok(Self {
            id: ArtifactId::new(),
            kind,
            path,
            content_hash: hash_bytes(bytes),
            provenance: Vec::new(),
            review_status: ReviewStatus::Draft,
        })
    }

    /// Returns a compact reference to this manifest.
    pub fn as_ref(&self) -> ArtifactRef {
        ArtifactRef {
            id: self.id,
            content_hash: self.content_hash.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArtifactManifest;
    use crate::ArtifactKind;

    #[test]
    fn manifest_can_be_serialized_to_json() {
        let manifest =
            ArtifactManifest::new(ArtifactKind::Csv, "artifacts/sample.csv", b"a,b\n1,2");

        match manifest {
            Ok(manifest) => {
                let serialized = serde_json::to_string(&manifest);
                assert!(serialized.is_ok());
            }
            Err(error) => panic!("manifest should accept non-empty paths: {error}"),
        }
    }
}
