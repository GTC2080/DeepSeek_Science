//! Tool call payloads.

use crate::{ToolId, ToolPermission};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One request to invoke a registered tool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Caller-provided call identifier for audit and result matching.
    pub call_id: String,
    /// Versioned tool identity.
    pub tool_id: ToolId,
    /// JSON input payload.
    pub arguments: Value,
    /// Permissions requested for this specific call.
    pub requested_permissions: Vec<ToolPermission>,
}

impl ToolCall {
    /// Creates a tool call.
    pub fn new(
        call_id: impl Into<String>,
        tool_id: ToolId,
        arguments: Value,
        requested_permissions: Vec<ToolPermission>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_id,
            arguments,
            requested_permissions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ToolCall;
    use crate::{ToolId, ToolPermission};
    use serde_json::json;

    #[test]
    fn tool_call_records_identity_and_requested_permissions() {
        let call = ToolCall::new(
            "call-1",
            ToolId::new("file.read", "1.0.0"),
            json!({"path": "README.md"}),
            vec![ToolPermission::ReadProjectFile],
        );

        assert_eq!(call.call_id, "call-1");
        assert_eq!(call.tool_id, ToolId::new("file.read", "1.0.0"));
        assert_eq!(call.arguments, json!({"path": "README.md"}));
        assert_eq!(
            call.requested_permissions,
            vec![ToolPermission::ReadProjectFile]
        );
    }
}
