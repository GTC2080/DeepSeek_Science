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
