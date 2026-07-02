//! Tool definition metadata.

use crate::ToolPermission;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Static metadata for a callable tool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name.
    pub name: String,
    /// Human-readable tool description.
    pub description: String,
    /// JSON schema for input arguments.
    pub input_schema: Value,
    /// JSON schema for output values.
    pub output_schema: Value,
    /// Permission policy that gates execution.
    pub permission: ToolPermission,
}

impl ToolDefinition {
    /// Creates a tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        output_schema: Value,
        permission: ToolPermission,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            output_schema,
            permission,
        }
    }
}
