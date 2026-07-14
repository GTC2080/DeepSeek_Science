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

impl ArtifactKind {
    /// Returns the stable lowercase label used by artifact envelopes.
    ///
    /// This accessor does not change the enum's existing serde representation.
    pub fn machine_label(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Figure => "figure",
            Self::Report => "report",
            Self::Code => "code",
            Self::Json => "json",
            Self::Text => "text",
            Self::Log => "log",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArtifactKind;

    #[test]
    fn every_kind_has_a_stable_machine_label() {
        let cases = [
            (ArtifactKind::Table, "table"),
            (ArtifactKind::Figure, "figure"),
            (ArtifactKind::Report, "report"),
            (ArtifactKind::Code, "code"),
            (ArtifactKind::Json, "json"),
            (ArtifactKind::Text, "text"),
            (ArtifactKind::Log, "log"),
            (ArtifactKind::Unknown, "unknown"),
        ];

        for (kind, expected) in cases {
            assert_eq!(kind.machine_label(), expected);
        }
    }

    #[test]
    fn existing_serde_representation_is_unchanged() {
        let serialized = serde_json::to_string(&ArtifactKind::Json).expect("kind should serialize");
        let deserialized: ArtifactKind =
            serde_json::from_str(&serialized).expect("kind should deserialize");

        assert_eq!(serialized, "\"Json\"");
        assert_eq!(deserialized, ArtifactKind::Json);
    }
}
