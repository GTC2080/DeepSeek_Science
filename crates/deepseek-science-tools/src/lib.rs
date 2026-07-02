#![forbid(unsafe_code)]
//! Tool protocol and registry for the headless kernel.
//!
//! Phase 1 only describes tools and permissions. It does not execute external
//! commands or ship heavy scientific integrations.

pub mod call;
pub mod definition;
pub mod error;
pub mod permissions;
pub mod registry;
pub mod result;

pub use call::ToolCall;
pub use definition::ToolDefinition;
pub use error::ToolError;
pub use permissions::{RiskLevel, ToolPermission};
pub use registry::ToolRegistry;
pub use result::{ToolResult, ToolStatus};
