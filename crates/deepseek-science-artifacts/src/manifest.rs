//! Artifact manifest and provenance records.

use crate::{ArtifactError, ArtifactKind, ReviewStatus};
use deepseek_science_core::ArtifactId;
use serde::{Deserialize, Serialize};

/// Stable reference to an artifact without loading the full manifest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Artifact identifier.
    pub id: ArtifactId,
    /// Coarse artifact kind.
    pub kind: ArtifactKind,
}

impl ArtifactRef {
    /// Creates a compact artifact reference.
    pub fn new(id: ArtifactId, kind: ArtifactKind) -> Self {
        Self { id, kind }
    }
}

/// One generic provenance record linking an artifact to upstream activity.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// Opaque run identifier, kept as a string to avoid runtime coupling.
    pub run_id: Option<String>,
    /// Opaque tool call identifier, when a tool produced or transformed data.
    pub tool_call_id: Option<String>,
    /// Opaque model call identifier, when a model produced or reviewed data.
    pub model_call_id: Option<String>,
    /// Prompt prefix hash associated with the producing model call.
    pub prompt_prefix_hash: Option<String>,
    /// Source artifact identifiers used to produce this artifact.
    pub source_artifact_ids: Vec<ArtifactId>,
    /// Short provenance note.
    pub note: Option<String>,
}

impl ProvenanceRecord {
    /// Creates a provenance note.
    pub fn new(note: impl Into<String>) -> Self {
        Self {
            note: Some(note.into()),
            ..Self::default()
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
    /// Optional human-readable title or label.
    pub title: Option<String>,
    /// Hashes of input files or input artifacts used to produce this artifact.
    pub input_hashes: Vec<String>,
    /// BLAKE3 hash of artifact content.
    pub content_hash: String,
    /// Provenance chain for audit and replay.
    pub provenance: Vec<ProvenanceRecord>,
    /// Current review status.
    pub review_status: ReviewStatus,
}

impl ArtifactManifest {
    /// Creates a manifest from a precomputed content hash.
    pub fn new(kind: ArtifactKind, content_hash: impl Into<String>) -> Result<Self, ArtifactError> {
        let content_hash = content_hash.into();
        if content_hash.trim().is_empty() {
            return Err(ArtifactError::EmptyContentHash);
        }

        Ok(Self {
            id: ArtifactId::new(),
            kind,
            title: None,
            input_hashes: Vec::new(),
            content_hash,
            provenance: Vec::new(),
            review_status: ReviewStatus::default(),
        })
    }

    /// Returns a compact reference to this manifest.
    pub fn as_ref(&self) -> ArtifactRef {
        ArtifactRef::new(self.id, self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtifactManifest, ArtifactRef, ProvenanceRecord};
    use crate::{hash_bytes, ArtifactKind, ReviewStatus};
    use deepseek_science_core::ArtifactId;

    #[test]
    fn artifact_ref_can_be_constructed() {
        let id = ArtifactId::new();
        let artifact_ref = ArtifactRef::new(id, ArtifactKind::Table);

        assert_eq!(artifact_ref.id, id);
        assert_eq!(artifact_ref.kind, ArtifactKind::Table);
    }

    #[test]
    fn manifest_can_be_constructed_with_generic_kind() {
        let content_hash = hash_bytes(b"report");
        let manifest =
            ArtifactManifest::new(ArtifactKind::Report, content_hash.clone()).expect("valid hash");

        assert_eq!(manifest.kind, ArtifactKind::Report);
        assert_eq!(manifest.content_hash, content_hash);
    }

    #[test]
    fn manifest_preserves_input_hashes() {
        let input_hash = hash_bytes(b"input table");
        let content_hash = hash_bytes(b"derived report");
        let mut manifest =
            ArtifactManifest::new(ArtifactKind::Report, content_hash).expect("valid hash");

        manifest.input_hashes.push(input_hash.clone());

        assert_eq!(manifest.input_hashes, vec![input_hash]);
    }

    #[test]
    fn manifest_preserves_provenance_records() {
        let source_id = ArtifactId::new();
        let content_hash = hash_bytes(b"derived json");
        let mut provenance = ProvenanceRecord::new("tool output imported");
        provenance.run_id = Some("run-001".to_string());
        provenance.tool_call_id = Some("tool-call-001".to_string());
        provenance.model_call_id = Some("model-call-001".to_string());
        provenance.prompt_prefix_hash = Some(hash_bytes(b"stable prompt"));
        provenance.source_artifact_ids.push(source_id);
        let mut manifest =
            ArtifactManifest::new(ArtifactKind::Json, content_hash).expect("valid hash");

        manifest.provenance.push(provenance.clone());

        assert_eq!(manifest.provenance, vec![provenance]);
    }

    #[test]
    fn review_status_defaults_to_not_reviewed() {
        let manifest =
            ArtifactManifest::new(ArtifactKind::Text, hash_bytes(b"text")).expect("valid hash");

        assert_eq!(ReviewStatus::default(), ReviewStatus::NotReviewed);
        assert_eq!(manifest.review_status, ReviewStatus::NotReviewed);
    }

    #[test]
    fn manifest_json_round_trip_preserves_metadata() {
        let input_hash = hash_bytes(b"source");
        let source_id = ArtifactId::new();
        let mut provenance = ProvenanceRecord::new("model summary reviewed");
        provenance.run_id = Some("run-002".to_string());
        provenance.source_artifact_ids.push(source_id);
        let mut manifest =
            ArtifactManifest::new(ArtifactKind::Text, hash_bytes(b"summary")).expect("valid hash");
        manifest.title = Some("Summary".to_string());
        manifest.input_hashes.push(input_hash);
        manifest.provenance.push(provenance);
        manifest.review_status = ReviewStatus::PassedWithWarnings;

        let serialized = serde_json::to_string(&manifest).expect("manifest should serialize");
        let deserialized: ArtifactManifest =
            serde_json::from_str(&serialized).expect("manifest should deserialize");

        assert_eq!(deserialized, manifest);
    }
}
