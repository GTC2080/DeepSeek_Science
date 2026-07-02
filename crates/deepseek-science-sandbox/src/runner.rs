//! Future sandbox runner boundary.

use crate::SandboxError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Request passed to a future sandbox runner.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxRequest {
    /// Command name or executable identifier.
    pub command: String,
    /// Command arguments.
    pub args: Vec<String>,
    /// Working directory requested by the caller.
    pub working_dir: PathBuf,
    /// Whether the command asks for network access.
    pub needs_network: bool,
}

/// Result returned by a future sandbox runner.
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
