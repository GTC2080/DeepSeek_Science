//! Review status for generated artifacts.

use serde::{Deserialize, Serialize};

/// Human or automated review state for an artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReviewStatus {
    /// Artifact exists but has not been reviewed.
    Draft,
    /// Artifact has been reviewed and accepted.
    Accepted,
    /// Artifact has been reviewed and rejected.
    Rejected,
}
