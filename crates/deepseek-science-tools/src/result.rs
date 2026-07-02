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
    /// Logical artifact references produced by the call handler.
    pub artifact_refs: Vec<String>,
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
            artifact_refs: Vec::new(),
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolResult, ToolStatus};
    use serde_json::json;

    #[test]
    fn successful_result_keeps_call_id_output_and_empty_artifacts() {
        let result = ToolResult::succeeded("call-1", json!({"answer": 42}));

        assert_eq!(result.call_id, "call-1");
        assert_eq!(result.status, ToolStatus::Succeeded);
        assert_eq!(result.output, Some(json!({"answer": 42})));
        assert!(result.artifact_refs.is_empty());
    }
}
