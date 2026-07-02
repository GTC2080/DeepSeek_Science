//! Error types for tool registration and calls.

use crate::{definition::ToolId, permissions::ToolPermission};
use thiserror::Error;

/// Errors produced by the tool registry.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ToolError {
    /// A tool identity was registered more than once.
    #[error("tool already registered: {tool_id}")]
    DuplicateTool {
        /// Duplicate tool identity.
        tool_id: ToolId,
    },

    /// A requested tool is not present in the registry.
    #[error("tool not found: {tool_id}")]
    ToolNotFound {
        /// Missing tool identity.
        tool_id: ToolId,
    },

    /// A tool name was empty or whitespace.
    #[error("tool name must not be empty")]
    InvalidToolName,

    /// A tool version was empty or whitespace.
    #[error("tool version must not be empty for {name}")]
    InvalidToolVersion {
        /// Tool name attached to the invalid version.
        name: String,
    },

    /// A required tool description was empty or whitespace.
    #[error("tool description must not be empty for {tool_id}")]
    InvalidToolDescription {
        /// Tool identity attached to the invalid description.
        tool_id: ToolId,
    },

    /// A tool is marked as forbidden.
    #[error("tool is forbidden: {tool_id}")]
    ForbiddenTool {
        /// Forbidden tool identity.
        tool_id: ToolId,
    },

    /// A requested permission is not allowed by policy.
    #[error("permission denied for tool {tool_id}: {permission:?}")]
    PermissionDenied {
        /// Tool identity whose permission was denied.
        tool_id: ToolId,
        /// Permission denied by a future policy layer.
        permission: ToolPermission,
    },
}
