//! Tool call payloads.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One request to invoke a registered tool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Caller-provided call identifier for audit and result matching.
    pub call_id: String,
    /// Registered tool name.
    pub tool_name: String,
    /// JSON input payload.
    pub input: Value,
}

impl ToolCall {
    /// Creates a tool call.
    pub fn new(call_id: impl Into<String>, tool_name: impl Into<String>, input: Value) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            input,
        }
    }
}
