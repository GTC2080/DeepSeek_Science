//! Tool result payloads.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Status returned by a tool call.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ToolStatus {
    /// Tool completed successfully.
    Succeeded,
    /// Tool failed in a controlled way.
    Failed,
    /// Tool was denied by policy.
    Denied,
    /// Tool cannot run until the user approves it.
    NeedsApproval,
}

/// Result returned after a tool call is handled.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// Call identifier this result answers.
    pub call_id: String,
    /// Execution status.
    pub status: ToolStatus,
    /// JSON output when the call succeeds.
    pub output: Option<Value>,
    /// Human-readable error when the call fails or is denied.
    pub error: Option<String>,
}

impl ToolResult {
    /// Creates a successful tool result.
    pub fn succeeded(call_id: impl Into<String>, output: Value) -> Self {
        Self {
            call_id: call_id.into(),
            status: ToolStatus::Succeeded,
            output: Some(output),
            error: None,
        }
    }
}
