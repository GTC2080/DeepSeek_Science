//! Tool permission and risk metadata.

use serde::{Deserialize, Serialize};

/// Coarse risk level used before a tool call is approved.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Pure read or deterministic local computation.
    Safe,
    /// Writes inside a project-approved output area.
    ProjectWrite,
    /// Accesses network or project-external resources.
    ExternalAccess,
    /// May delete, overwrite, or otherwise damage user data.
    Destructive,
}

/// Permission policy attached to a tool definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolPermission {
    /// Declared tool risk.
    pub risk_level: RiskLevel,
    /// Whether a user approval is required before execution.
    pub requires_approval: bool,
}

impl ToolPermission {
    /// Creates a permission policy.
    pub fn new(risk_level: RiskLevel, requires_approval: bool) -> Self {
        Self {
            risk_level,
            requires_approval,
        }
    }

    /// Returns true when the policy requires approval before execution.
    pub fn needs_approval(&self) -> bool {
        self.requires_approval || self.risk_level != RiskLevel::Safe
    }
}

#[cfg(test)]
mod tests {
    use super::{RiskLevel, ToolPermission};

    #[test]
    fn safe_vs_needs_approval_risk_levels() {
        let safe = ToolPermission::new(RiskLevel::Safe, false);
        let write = ToolPermission::new(RiskLevel::ProjectWrite, false);

        assert!(!safe.needs_approval());
        assert!(write.needs_approval());
    }
}
