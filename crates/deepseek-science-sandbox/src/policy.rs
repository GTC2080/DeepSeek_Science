//! Sandbox permission policy.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Permission category requested by a future runner.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExecutionPermission {
    /// Read files inside approved project roots.
    ReadProjectFile,
    /// Write files inside approved project output roots.
    WriteProjectFile,
    /// Access the network.
    Network,
    /// Spawn a subprocess.
    Subprocess,
}

/// Policy used to decide whether a runner may access files, network, or subprocesses.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Whether network access is allowed.
    pub allow_network: bool,
    /// Whether subprocess execution is allowed.
    pub allow_subprocess: bool,
    /// Project roots allowed for file access.
    pub allowed_roots: Vec<PathBuf>,
}

impl SandboxPolicy {
    /// Creates the default deny-by-default policy.
    pub fn deny_by_default() -> Self {
        Self {
            allow_network: false,
            allow_subprocess: false,
            allowed_roots: Vec::new(),
        }
    }

    /// Returns true when the policy allows the requested permission.
    pub fn allows_permission(&self, permission: ExecutionPermission) -> bool {
        match permission {
            ExecutionPermission::ReadProjectFile | ExecutionPermission::WriteProjectFile => {
                !self.allowed_roots.is_empty()
            }
            ExecutionPermission::Network => self.allow_network,
            ExecutionPermission::Subprocess => self.allow_subprocess,
        }
    }

    /// Returns true when a path is inside one of the approved project roots.
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
    use super::{ExecutionPermission, SandboxPolicy};

    #[test]
    fn default_policy_forbids_network() {
        let policy = SandboxPolicy::default();

        assert!(!policy.allows_permission(ExecutionPermission::Network));
    }

    #[test]
    fn default_policy_forbids_project_external_file_access() {
        let policy = SandboxPolicy::default();

        assert!(!policy.allows_path("/outside/project/file.txt"));
    }
}
