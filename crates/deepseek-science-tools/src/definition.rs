//! Tool definition metadata.

use crate::{RiskLevel, ToolError, ToolPermission};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Stable identity for one versioned tool definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ToolId {
    /// Registered tool name.
    pub name: String,
    /// Tool definition version.
    pub version: String,
}

impl ToolId {
    /// Creates a tool identity from name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    /// Validates the identity fields used as registry keys.
    pub fn validate(&self) -> Result<(), ToolError> {
        if self.name.trim().is_empty() {
            return Err(ToolError::InvalidToolName);
        }

        if self.version.trim().is_empty() {
            return Err(ToolError::InvalidToolVersion {
                name: self.name.clone(),
            });
        }

        Ok(())
    }
}

impl fmt::Display for ToolId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}", self.name, self.version)
    }
}

/// Static metadata for a callable tool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name.
    pub name: String,
    /// Tool definition version.
    pub version: String,
    /// Human-readable tool description.
    pub description: String,
    /// JSON schema for input arguments.
    pub input_schema: Value,
    /// JSON schema for output values.
    pub output_schema: Value,
    /// Generic permission boundaries required by this tool.
    pub required_permissions: Vec<ToolPermission>,
    /// Coarse risk metadata used by approval and sandbox policy.
    pub risk_level: RiskLevel,
}

impl ToolDefinition {
    /// Creates a tool definition.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        output_schema: Value,
        required_permissions: Vec<ToolPermission>,
        risk_level: RiskLevel,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            input_schema,
            output_schema,
            required_permissions,
            risk_level,
        }
    }

    /// Returns the stable name and version identity for this definition.
    pub fn identity(&self) -> ToolId {
        ToolId::new(self.name.clone(), self.version.clone())
    }

    /// Validates registry key fields without interpreting schemas.
    pub fn validate(&self) -> Result<(), ToolError> {
        let identity = self.identity();
        identity.validate()?;

        if self.description.trim().is_empty() {
            return Err(ToolError::InvalidToolDescription { tool_id: identity });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ToolDefinition;
    use crate::{RiskLevel, ToolId, ToolPermission};
    use serde_json::json;

    #[test]
    fn definition_exposes_stable_identity_and_permissions() {
        let definition = ToolDefinition::new(
            "file.read",
            "1.0.0",
            "read a project file",
            json!({"type": "object"}),
            json!({"type": "object"}),
            vec![ToolPermission::ReadProjectFile],
            RiskLevel::Safe,
        );

        assert_eq!(definition.identity(), ToolId::new("file.read", "1.0.0"));
        assert_eq!(
            definition.required_permissions,
            vec![ToolPermission::ReadProjectFile]
        );
        assert_eq!(definition.risk_level, RiskLevel::Safe);
    }
}
