//! Tool permission boundaries and risk metadata.

use serde::{Deserialize, Serialize};

/// Coarse risk level used before a tool call is approved.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Pure metadata or deterministic local computation that can run without approval.
    Safe,
    /// Requires explicit approval before a future runner may execute it.
    NeedsApproval,
    /// High-impact capability that requires approval and stronger sandboxing.
    Dangerous,
    /// Must not be executed by the registry consumer.
    Forbidden,
}

impl RiskLevel {
    /// Returns true when a future runner must collect approval before execution.
    pub fn requires_approval(self) -> bool {
        matches!(self, Self::NeedsApproval | Self::Dangerous)
    }

    /// Returns true when this risk level is allowed without approval.
    pub fn is_executable_without_approval(self) -> bool {
        self == Self::Safe
    }

    /// Returns true when this tool must not be executed.
    pub fn is_forbidden(self) -> bool {
        self == Self::Forbidden
    }
}

/// Generic capability boundary requested by a tool definition or call.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ToolPermission {
    /// Read a file inside the current project boundary.
    ReadProjectFile,
    /// Write a file inside the current project boundary.
    WriteProjectFile,
    /// Access the network.
    AccessNetwork,
    /// Execute a subprocess.
    ExecuteSubprocess,
    /// Read environment variables.
    AccessEnvironment,
    /// Access paths outside the current project boundary.
    AccessProjectExternalPath,
}

#[cfg(test)]
mod tests {
    use super::RiskLevel;

    #[test]
    fn safe_risk_does_not_require_approval() {
        assert!(!RiskLevel::Safe.requires_approval());
        assert!(RiskLevel::Safe.is_executable_without_approval());
        assert!(!RiskLevel::Safe.is_forbidden());
    }

    #[test]
    fn approval_risks_require_approval() {
        assert!(RiskLevel::NeedsApproval.requires_approval());
        assert!(RiskLevel::Dangerous.requires_approval());
        assert!(!RiskLevel::NeedsApproval.is_executable_without_approval());
        assert!(!RiskLevel::Dangerous.is_executable_without_approval());
    }

    #[test]
    fn forbidden_risk_is_forbidden() {
        assert!(RiskLevel::Forbidden.is_forbidden());
        assert!(!RiskLevel::Forbidden.is_executable_without_approval());
    }
}
