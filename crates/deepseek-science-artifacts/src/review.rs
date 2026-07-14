//! Review status for generated or imported artifacts.

use serde::{Deserialize, Serialize};

/// Human or automated review state for an artifact.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReviewStatus {
    /// Artifact exists but no review result has been recorded.
    #[default]
    NotReviewed,
    /// Artifact passed review without warnings.
    Passed,
    /// Artifact passed review with warnings that should remain visible.
    PassedWithWarnings,
    /// Artifact failed review.
    Failed,
}

impl ReviewStatus {
    /// Returns the stable lowercase label used by artifact envelopes.
    ///
    /// This accessor does not change the enum's existing serde representation.
    pub fn machine_label(self) -> &'static str {
        match self {
            Self::NotReviewed => "not_reviewed",
            Self::Passed => "passed",
            Self::PassedWithWarnings => "passed_with_warnings",
            Self::Failed => "failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReviewStatus;

    #[test]
    fn every_review_status_has_a_stable_machine_label() {
        let cases = [
            (ReviewStatus::NotReviewed, "not_reviewed"),
            (ReviewStatus::Passed, "passed"),
            (ReviewStatus::PassedWithWarnings, "passed_with_warnings"),
            (ReviewStatus::Failed, "failed"),
        ];

        for (status, expected) in cases {
            assert_eq!(status.machine_label(), expected);
        }
    }

    #[test]
    fn existing_serde_representation_is_unchanged() {
        let serialized = serde_json::to_string(&ReviewStatus::PassedWithWarnings)
            .expect("status should serialize");
        let deserialized: ReviewStatus =
            serde_json::from_str(&serialized).expect("status should deserialize");

        assert_eq!(serialized, "\"PassedWithWarnings\"");
        assert_eq!(deserialized, ReviewStatus::PassedWithWarnings);
    }
}
