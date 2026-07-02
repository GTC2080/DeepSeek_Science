//! Future sandbox runner boundary.

use crate::{ExecutionPermission, SandboxError};
use serde::{Deserialize, Serialize};

/// Request passed to a future sandbox runner.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxRequest {
    /// Stable logical request label supplied by the caller.
    pub label: String,
    /// Permissions requested before any side effect may occur.
    pub permissions: Vec<ExecutionPermission>,
    /// Optional human-readable reason for audit or approval surfaces.
    pub reason: Option<String>,
    /// Optional caller-provided risk label kept as inert metadata.
    pub risk_label: Option<String>,
}

impl SandboxRequest {
    /// Creates an inert sandbox request from a stable label and permissions.
    pub fn new(label: impl Into<String>, permissions: Vec<ExecutionPermission>) -> Self {
        Self {
            label: label.into(),
            permissions,
            reason: None,
            risk_label: None,
        }
    }
}

/// Result returned by a future sandbox runner after execution is implemented.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxResult {
    /// Process exit code when execution is supported.
    pub exit_code: i32,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

/// Interface future runners must implement after policy checks.
pub trait SandboxRunner {
    /// Runs a sandbox request or returns a policy/execution error.
    fn run(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError>;
}

#[cfg(test)]
mod tests {
    use super::{SandboxRequest, SandboxResult, SandboxRunner};
    use crate::{ExecutionPermission, SandboxDecision, SandboxError, SandboxPolicy};

    struct InterfaceOnlyRunner {
        policy: SandboxPolicy,
    }

    impl SandboxRunner for InterfaceOnlyRunner {
        fn run(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError> {
            match self.policy.evaluate(&request) {
                SandboxDecision::Allowed { reason } => {
                    Err(SandboxError::UnsupportedExecution { reason })
                }
                SandboxDecision::RequiresApproval {
                    permissions,
                    reason,
                } => Err(SandboxError::ApprovalRequired {
                    permissions,
                    reason,
                }),
                SandboxDecision::Denied {
                    permissions,
                    reason,
                } => Err(SandboxError::PermissionDenied {
                    permissions,
                    reason,
                }),
            }
        }
    }

    #[test]
    fn sandbox_runner_trait_does_not_execute_by_itself() {
        let runner = InterfaceOnlyRunner {
            policy: SandboxPolicy::default(),
        };
        let request = SandboxRequest::new("noop", Vec::new());

        assert_eq!(
            runner.run(request),
            Err(SandboxError::UnsupportedExecution {
                reason: "request does not ask for sandbox permissions".to_owned(),
            })
        );
    }

    #[test]
    fn sandbox_runner_trait_can_surface_policy_denials() {
        let runner = InterfaceOnlyRunner {
            policy: SandboxPolicy::default(),
        };
        let request = SandboxRequest::new("network", vec![ExecutionPermission::AccessNetwork]);

        assert_eq!(
            runner.run(request),
            Err(SandboxError::PermissionDenied {
                permissions: vec![ExecutionPermission::AccessNetwork],
                reason: "permission is denied by sandbox policy".to_owned(),
            })
        );
    }
}
