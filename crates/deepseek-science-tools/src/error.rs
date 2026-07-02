//! Error types for tool registration and calls.

use thiserror::Error;

/// Errors produced by the tool registry.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ToolError {
    /// A tool name was registered more than once.
    #[error("tool already registered: {name}")]
    DuplicateTool {
        /// Duplicate tool name.
        name: String,
    },

    /// A requested tool is not present in the registry.
    #[error("tool not found: {name}")]
    ToolNotFound {
        /// Missing tool name.
        name: String,
    },
}
