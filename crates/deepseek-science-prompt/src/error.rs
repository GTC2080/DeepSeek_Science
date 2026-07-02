//! Error types for prompt compilation.

use thiserror::Error;

/// Errors produced while compiling prompt sections.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PromptError {
    /// Stable prompt template version was empty.
    #[error("prompt template version must not be empty")]
    EmptyTemplateVersion,
    /// A stable section had no name.
    #[error("prompt section name must not be empty")]
    EmptySectionName,
}
