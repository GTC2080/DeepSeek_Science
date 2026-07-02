//! In-memory tool registry metadata.

use crate::{ToolDefinition, ToolError, ToolId};
use std::collections::BTreeMap;

/// Deterministic registry of known tool definitions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolRegistry {
    tools: BTreeMap<ToolId, ToolDefinition>,
}

impl ToolRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new tool definition.
    pub fn register(&mut self, definition: ToolDefinition) -> Result<(), ToolError> {
        definition.validate()?;

        let tool_id = definition.identity();
        if self.tools.contains_key(&tool_id) {
            return Err(ToolError::DuplicateTool { tool_id });
        }

        self.tools.insert(tool_id, definition);
        Ok(())
    }

    /// Returns a tool definition by name and version.
    pub fn get(&self, name: &str, version: &str) -> Result<&ToolDefinition, ToolError> {
        self.get_by_id(&ToolId::new(name, version))
    }

    /// Returns a tool definition by stable identity.
    pub fn get_by_id(&self, tool_id: &ToolId) -> Result<&ToolDefinition, ToolError> {
        self.tools
            .get(tool_id)
            .ok_or_else(|| ToolError::ToolNotFound {
                tool_id: tool_id.clone(),
            })
    }

    /// Returns a registered tool unless its risk level is forbidden.
    pub fn ensure_not_forbidden(&self, tool_id: &ToolId) -> Result<&ToolDefinition, ToolError> {
        let definition = self.get_by_id(tool_id)?;
        if definition.risk_level.is_forbidden() {
            return Err(ToolError::ForbiddenTool {
                tool_id: tool_id.clone(),
            });
        }

        Ok(definition)
    }

    /// Lists registered tools in stable identity order.
    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// Returns the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns true when no tools are registered.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::ToolRegistry;
    use crate::{RiskLevel, ToolDefinition, ToolError, ToolId, ToolPermission};
    use serde_json::json;

    fn definition(name: &str, version: &str) -> ToolDefinition {
        ToolDefinition::new(
            name,
            version,
            "local calculation",
            json!({"type": "object"}),
            json!({"type": "object"}),
            vec![ToolPermission::ReadProjectFile],
            RiskLevel::Safe,
        )
    }

    #[test]
    fn register_safe_tool() {
        let mut registry = ToolRegistry::new();

        let result = registry.register(definition("math.mean", "1.0.0"));

        assert!(result.is_ok());
        assert!(registry.get("math.mean", "1.0.0").is_ok());
    }

    #[test]
    fn duplicate_tool_identity_returns_error() {
        let mut registry = ToolRegistry::new();
        let first = registry.register(definition("math.mean", "1.0.0"));
        let second = registry.register(definition("math.mean", "1.0.0"));

        assert!(first.is_ok());
        assert_eq!(
            second,
            Err(ToolError::DuplicateTool {
                tool_id: ToolId::new("math.mean", "1.0.0")
            })
        );
    }

    #[test]
    fn lookup_registered_tool_succeeds_by_identity() {
        let mut registry = ToolRegistry::new();
        registry
            .register(definition("math.mean", "1.0.0"))
            .expect("test setup should register a unique tool");

        let tool = registry
            .get_by_id(&ToolId::new("math.mean", "1.0.0"))
            .expect("registered tool should be found");

        assert_eq!(tool.name, "math.mean");
        assert_eq!(tool.version, "1.0.0");
    }

    #[test]
    fn lookup_missing_tool_returns_structured_error() {
        let registry = ToolRegistry::new();

        assert_eq!(
            registry.get("missing.tool", "1.0.0"),
            Err(ToolError::ToolNotFound {
                tool_id: ToolId::new("missing.tool", "1.0.0")
            })
        );
    }

    #[test]
    fn list_registered_tools_is_deterministic() {
        let mut registry = ToolRegistry::new();
        registry
            .register(definition("z.tool", "1.0.0"))
            .expect("test setup should register z.tool");
        registry
            .register(definition("a.tool", "1.0.0"))
            .expect("test setup should register a.tool");
        registry
            .register(definition("a.tool", "2.0.0"))
            .expect("test setup should register a second version");

        let identities: Vec<_> = registry
            .list()
            .into_iter()
            .map(ToolDefinition::identity)
            .collect();

        assert_eq!(
            identities,
            vec![
                ToolId::new("a.tool", "1.0.0"),
                ToolId::new("a.tool", "2.0.0"),
                ToolId::new("z.tool", "1.0.0"),
            ]
        );
    }

    #[test]
    fn permission_list_is_preserved() {
        let mut definition = definition("file.copy", "1.0.0");
        definition.required_permissions = vec![
            ToolPermission::ReadProjectFile,
            ToolPermission::WriteProjectFile,
        ];

        let mut registry = ToolRegistry::new();
        registry
            .register(definition)
            .expect("test setup should register file.copy");

        let tool = registry
            .get("file.copy", "1.0.0")
            .expect("registered tool should be found");

        assert_eq!(
            tool.required_permissions,
            vec![
                ToolPermission::ReadProjectFile,
                ToolPermission::WriteProjectFile,
            ]
        );
    }

    #[test]
    fn empty_tool_name_is_rejected() {
        let mut registry = ToolRegistry::new();

        assert_eq!(
            registry.register(definition("", "1.0.0")),
            Err(ToolError::InvalidToolName)
        );
    }

    #[test]
    fn empty_tool_version_is_rejected() {
        let mut registry = ToolRegistry::new();

        assert_eq!(
            registry.register(definition("math.mean", "")),
            Err(ToolError::InvalidToolVersion {
                name: "math.mean".to_owned()
            })
        );
    }

    #[test]
    fn empty_tool_description_is_rejected() {
        let mut definition = definition("math.mean", "1.0.0");
        definition.description.clear();

        let mut registry = ToolRegistry::new();

        assert_eq!(
            registry.register(definition),
            Err(ToolError::InvalidToolDescription {
                tool_id: ToolId::new("math.mean", "1.0.0")
            })
        );
    }

    #[test]
    fn forbidden_tool_returns_structured_error() {
        let mut definition = definition("shell.run", "1.0.0");
        definition.risk_level = RiskLevel::Forbidden;

        let mut registry = ToolRegistry::new();
        registry
            .register(definition)
            .expect("test setup should register forbidden metadata");

        assert_eq!(
            registry.ensure_not_forbidden(&ToolId::new("shell.run", "1.0.0")),
            Err(ToolError::ForbiddenTool {
                tool_id: ToolId::new("shell.run", "1.0.0")
            })
        );
    }
}
