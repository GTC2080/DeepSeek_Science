//! Sandbox permission policy.

use crate::runner::SandboxRequest;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const EMPTY_REQUEST_REASON: &str = "request does not ask for sandbox permissions";
const ALLOWED_REASON: &str = "all requested permissions are allowed by sandbox policy";
const APPROVAL_REASON: &str = "permission requires explicit approval";
const DENIED_REASON: &str = "permission is denied by sandbox policy";

/// Generic execution boundary requested by a future runner.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExecutionPermission {
    /// Read a logical path inside the current project boundary.
    ReadProjectPath,
    /// Write a logical path inside the current project boundary.
    WriteProjectPath,
    /// Read a logical path outside the current project boundary.
    ReadProjectExternalPath,
    /// Write a logical path outside the current project boundary.
    WriteProjectExternalPath,
    /// Access the network.
    AccessNetwork,
    /// Spawn a subprocess.
    ExecuteSubprocess,
    /// Read environment variables.
    ReadEnvironment,
    /// Write environment variables.
    WriteEnvironment,
}

/// Approval state required before a permission can be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ApprovalRequirement {
    /// The permission is allowed without human approval.
    NotRequired,
    /// The permission may proceed only after explicit approval.
    Required,
    /// The permission is forbidden by policy and cannot be approved.
    Forbidden,
}

/// Deterministic result of evaluating a sandbox request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SandboxDecision {
    /// All requested permissions are allowed.
    Allowed {
        /// Explanation safe for logs, UI, or audit records.
        reason: String,
    },
    /// One or more requested permissions require approval.
    RequiresApproval {
        /// Permissions that require explicit approval.
        permissions: Vec<ExecutionPermission>,
        /// Explanation safe for logs, UI, or audit records.
        reason: String,
    },
    /// One or more requested permissions are forbidden.
    Denied {
        /// Permissions denied by the sandbox policy.
        permissions: Vec<ExecutionPermission>,
        /// Explanation safe for logs, UI, or audit records.
        reason: String,
    },
}

impl SandboxDecision {
    /// Returns the approval requirement represented by this decision.
    pub fn approval_requirement(&self) -> ApprovalRequirement {
        match self {
            Self::Allowed { .. } => ApprovalRequirement::NotRequired,
            Self::RequiresApproval { .. } => ApprovalRequirement::Required,
            Self::Denied { .. } => ApprovalRequirement::Forbidden,
        }
    }
}

/// Policy used to decide whether a runner may access files, network, or subprocesses.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Whether network access is allowed.
    pub allow_network: bool,
    /// Whether subprocess execution is allowed.
    pub allow_subprocess: bool,
    /// Logical project roots used by [`Self::allows_path`].
    pub allowed_roots: Vec<PathBuf>,
    /// Permissions explicitly allowed without approval.
    pub allowed_permissions: Vec<ExecutionPermission>,
    /// Permissions that require explicit approval.
    pub approval_required_permissions: Vec<ExecutionPermission>,
    /// Permissions that cannot be approved under this policy.
    pub forbidden_permissions: Vec<ExecutionPermission>,
}

impl SandboxPolicy {
    /// Creates the default deny-by-default policy.
    pub fn deny_by_default() -> Self {
        Self {
            allow_network: false,
            allow_subprocess: false,
            allowed_roots: Vec::new(),
            allowed_permissions: Vec::new(),
            approval_required_permissions: Vec::new(),
            forbidden_permissions: Vec::new(),
        }
    }

    /// Evaluates a request without executing, writing, or inspecting the host.
    pub fn evaluate(&self, request: &SandboxRequest) -> SandboxDecision {
        if request.permissions.is_empty() {
            return SandboxDecision::Allowed {
                reason: EMPTY_REQUEST_REASON.to_owned(),
            };
        }

        let mut approval_required = Vec::new();
        let mut denied = Vec::new();

        for permission in request.permissions.iter().copied() {
            match self.requirement_for(permission) {
                ApprovalRequirement::NotRequired => {}
                ApprovalRequirement::Required => approval_required.push(permission),
                ApprovalRequirement::Forbidden => denied.push(permission),
            }
        }

        if !denied.is_empty() {
            return SandboxDecision::Denied {
                permissions: denied,
                reason: DENIED_REASON.to_owned(),
            };
        }

        if !approval_required.is_empty() {
            return SandboxDecision::RequiresApproval {
                permissions: approval_required,
                reason: APPROVAL_REASON.to_owned(),
            };
        }

        SandboxDecision::Allowed {
            reason: ALLOWED_REASON.to_owned(),
        }
    }

    /// Returns the approval requirement for one permission.
    pub fn requirement_for(&self, permission: ExecutionPermission) -> ApprovalRequirement {
        if self.forbidden_permissions.contains(&permission) {
            return ApprovalRequirement::Forbidden;
        }

        if self.approval_required_permissions.contains(&permission) {
            return ApprovalRequirement::Required;
        }

        if self.allows_permission_without_approval(permission) {
            return ApprovalRequirement::NotRequired;
        }

        ApprovalRequirement::Forbidden
    }

    /// Returns true when the policy allows the requested permission.
    pub fn allows_permission(&self, permission: ExecutionPermission) -> bool {
        self.requirement_for(permission) == ApprovalRequirement::NotRequired
    }

    fn allows_permission_without_approval(&self, permission: ExecutionPermission) -> bool {
        match permission {
            ExecutionPermission::AccessNetwork => {
                self.allow_network || self.allowed_permissions.contains(&permission)
            }
            ExecutionPermission::ExecuteSubprocess => {
                self.allow_subprocess || self.allowed_permissions.contains(&permission)
            }
            ExecutionPermission::ReadProjectPath
            | ExecutionPermission::WriteProjectPath
            | ExecutionPermission::ReadProjectExternalPath
            | ExecutionPermission::WriteProjectExternalPath
            | ExecutionPermission::ReadEnvironment
            | ExecutionPermission::WriteEnvironment => {
                self.allowed_permissions.contains(&permission)
            }
        }
    }

    /// Returns true when a logical path is inside one of the approved project roots.
    pub fn allows_path(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        self.allowed_roots.iter().any(|root| path.starts_with(root))
    }
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self::deny_by_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{ApprovalRequirement, ExecutionPermission, SandboxDecision, SandboxPolicy};

    #[test]
    fn default_policy_denies_network() {
        let policy = SandboxPolicy::default();
        let request =
            crate::SandboxRequest::new("network", vec![ExecutionPermission::AccessNetwork]);

        assert_eq!(
            policy.evaluate(&request),
            SandboxDecision::Denied {
                permissions: vec![ExecutionPermission::AccessNetwork],
                reason: "permission is denied by sandbox policy".to_owned(),
            }
        );
    }

    #[test]
    fn default_policy_denies_project_external_read() {
        let policy = SandboxPolicy::default();
        let request = crate::SandboxRequest::new(
            "external-read",
            vec![ExecutionPermission::ReadProjectExternalPath],
        );

        assert_eq!(
            policy.evaluate(&request),
            SandboxDecision::Denied {
                permissions: vec![ExecutionPermission::ReadProjectExternalPath],
                reason: "permission is denied by sandbox policy".to_owned(),
            }
        );
    }

    #[test]
    fn default_policy_denies_project_external_write() {
        let policy = SandboxPolicy::default();
        let request = crate::SandboxRequest::new(
            "external-write",
            vec![ExecutionPermission::WriteProjectExternalPath],
        );

        assert_eq!(
            policy.evaluate(&request),
            SandboxDecision::Denied {
                permissions: vec![ExecutionPermission::WriteProjectExternalPath],
                reason: "permission is denied by sandbox policy".to_owned(),
            }
        );
    }

    #[test]
    fn default_policy_denies_environment_access() {
        let policy = SandboxPolicy::default();
        let request = crate::SandboxRequest::new(
            "environment",
            vec![
                ExecutionPermission::ReadEnvironment,
                ExecutionPermission::WriteEnvironment,
            ],
        );

        assert_eq!(
            policy.evaluate(&request),
            SandboxDecision::Denied {
                permissions: vec![
                    ExecutionPermission::ReadEnvironment,
                    ExecutionPermission::WriteEnvironment,
                ],
                reason: "permission is denied by sandbox policy".to_owned(),
            }
        );
    }

    #[test]
    fn default_policy_denies_subprocess_execution() {
        let policy = SandboxPolicy::default();
        let request =
            crate::SandboxRequest::new("subprocess", vec![ExecutionPermission::ExecuteSubprocess]);

        assert_eq!(
            policy.evaluate(&request),
            SandboxDecision::Denied {
                permissions: vec![ExecutionPermission::ExecuteSubprocess],
                reason: "permission is denied by sandbox policy".to_owned(),
            }
        );
    }

    #[test]
    fn default_policy_allows_empty_request() {
        let policy = SandboxPolicy::default();
        let request = crate::SandboxRequest::new("noop", Vec::new());

        assert_eq!(
            policy.evaluate(&request),
            SandboxDecision::Allowed {
                reason: "request does not ask for sandbox permissions".to_owned(),
            }
        );
    }

    #[test]
    fn explicit_policy_can_require_approval_for_project_write() {
        let policy = SandboxPolicy {
            approval_required_permissions: vec![ExecutionPermission::WriteProjectPath],
            ..SandboxPolicy::default()
        };
        let request = crate::SandboxRequest::new(
            "write-project",
            vec![ExecutionPermission::WriteProjectPath],
        );

        assert_eq!(
            policy.evaluate(&request),
            SandboxDecision::RequiresApproval {
                permissions: vec![ExecutionPermission::WriteProjectPath],
                reason: "permission requires explicit approval".to_owned(),
            }
        );
        assert_eq!(
            policy.requirement_for(ExecutionPermission::WriteProjectPath),
            ApprovalRequirement::Required
        );
    }

    #[test]
    fn denied_decision_preserves_denied_permission() {
        let policy = SandboxPolicy::default();
        let request =
            crate::SandboxRequest::new("denied", vec![ExecutionPermission::AccessNetwork]);

        let SandboxDecision::Denied {
            permissions,
            reason,
        } = policy.evaluate(&request)
        else {
            panic!("network should be denied by default");
        };

        assert_eq!(permissions, vec![ExecutionPermission::AccessNetwork]);
        assert_eq!(reason, "permission is denied by sandbox policy");
    }
}
