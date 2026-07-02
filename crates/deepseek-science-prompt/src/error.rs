//! Error types for prompt compilation.

use thiserror::Error;

/// Errors produced while compiling prompt sections.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PromptError {
    /// A stable section had no name.
    #[error("prompt section name must not be empty")]
    EmptySectionName,
}
