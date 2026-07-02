//! In-memory tool registry metadata.

use crate::{ToolDefinition, ToolError};
use std::collections::BTreeMap;

/// Deterministic registry of known tool definitions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolRegistry {
    tools: BTreeMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new tool definition.
    pub fn register(&mut self, definition: ToolDefinition) -> Result<(), ToolError> {
        let name = definition.name.clone();
        if self.tools.contains_key(&name) {
            return Err(ToolError::DuplicateTool { name });
        }

        self.tools.insert(name, definition);
        Ok(())
    }

    /// Returns a tool definition by name.
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
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
    use crate::{RiskLevel, ToolDefinition, ToolPermission};
    use serde_json::json;

    fn definition(name: &str) -> ToolDefinition {
        ToolDefinition::new(
            name,
            "local calculation",
            json!({"type": "object"}),
            json!({"type": "object"}),
            ToolPermission::new(RiskLevel::Safe, false),
        )
    }

    #[test]
    fn register_tool() {
        let mut registry = ToolRegistry::new();

        let result = registry.register(definition("math.mean"));

        assert!(result.is_ok());
        assert!(registry.get("math.mean").is_some());
    }

    #[test]
    fn duplicate_tool_name_returns_error() {
        let mut registry = ToolRegistry::new();
        let first = registry.register(definition("math.mean"));
        let second = registry.register(definition("math.mean"));

        assert!(first.is_ok());
        assert!(second.is_err());
    }
}
