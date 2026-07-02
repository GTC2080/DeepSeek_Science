//! Prompt section types used by the prefix compiler.

use serde::{Deserialize, Serialize};

/// Category for a stable prompt section.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PromptSectionKind {
    /// System-level behavior and kernel rules.
    System,
    /// Safety, privacy, or approval policy text.
    Policy,
    /// Domain pack rules, kept out of the core crate.
    DomainRules,
    /// Tool names, schemas, and permission summaries.
    ToolManifest,
    /// Project-local context selected for a run.
    ProjectContext,
}

impl PromptSectionKind {
    /// Returns a deterministic label used in compiled prefix text.
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Policy => "policy",
            Self::DomainRules => "domain_rules",
            Self::ToolManifest => "tool_manifest",
            Self::ProjectContext => "project_context",
        }
    }
}

/// One stable section of a compiled prompt prefix.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptSection {
    /// Section category.
    pub kind: PromptSectionKind,
    /// Stable section name for audit and hashing.
    pub name: String,
    /// Section body. Trailing whitespace is ignored by the compiler.
    pub content: String,
}

impl PromptSection {
    /// Creates a prompt section.
    pub fn new(
        kind: PromptSectionKind,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            name: name.into(),
            content: content.into(),
        }
    }
}
