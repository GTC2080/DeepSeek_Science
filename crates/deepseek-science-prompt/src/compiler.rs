//! Deterministic prompt prefix compilation.

use crate::{hash::hash_stable_prefix, PromptError, PromptSection};
use serde::{Deserialize, Serialize};

/// Version metadata that contributes to prompt auditability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptVersionInfo {
    /// Kernel prompt contract version.
    pub kernel_version: String,
    /// Optional domain pack version.
    pub domain_pack_version: Option<String>,
    /// Optional tool manifest version.
    pub tool_manifest_version: Option<String>,
    /// Optional project context version.
    pub project_context_version: Option<String>,
}

impl PromptVersionInfo {
    /// Creates version metadata for the kernel prompt contract.
    pub fn new(kernel_version: impl Into<String>) -> Self {
        Self {
            kernel_version: kernel_version.into(),
            domain_pack_version: None,
            tool_manifest_version: None,
            project_context_version: None,
        }
    }
}

/// Input passed to the prompt compiler.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptCompileInput {
    /// Stable sections that should affect the prefix hash.
    pub stable_sections: Vec<PromptSection>,
    /// User request or other per-run tail content.
    pub user_request: String,
    /// Version metadata recorded with the compiled prompt.
    pub version_info: PromptVersionInfo,
}

impl PromptCompileInput {
    /// Creates compiler input from stable sections, user request, and versions.
    pub fn new(
        stable_sections: Vec<PromptSection>,
        user_request: impl Into<String>,
        version_info: PromptVersionInfo,
    ) -> Self {
        Self {
            stable_sections,
            user_request: user_request.into(),
            version_info,
        }
    }
}

/// Prompt compiled into stable and variable regions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledPrompt {
    /// Deterministic prefix suitable for cache reuse.
    pub stable_prefix: String,
    /// Variable user tail that must not affect the prefix hash.
    pub variable_tail: String,
    /// Full prompt sent to a model provider.
    pub full_prompt: String,
    /// BLAKE3 hash of `stable_prefix`.
    pub prefix_hash: String,
    /// Version metadata used to audit prompt provenance.
    pub version_info: PromptVersionInfo,
}

/// Compiles stable sections and a variable tail into a prompt.
pub fn compile_prompt(input: PromptCompileInput) -> Result<CompiledPrompt, PromptError> {
    let stable_prefix = compile_stable_prefix(&input.stable_sections)?;
    let variable_tail = format!("## user_request\n{}\n", input.user_request.trim_end());
    let full_prompt = format!("{stable_prefix}{variable_tail}");
    let prefix_hash = hash_stable_prefix(&stable_prefix);

    Ok(CompiledPrompt {
        stable_prefix,
        variable_tail,
        full_prompt,
        prefix_hash,
        version_info: input.version_info,
    })
}

fn compile_stable_prefix(sections: &[PromptSection]) -> Result<String, PromptError> {
    let mut prefix = String::new();

    for section in sections {
        let name = section.name.trim();
        if name.is_empty() {
            return Err(PromptError::EmptySectionName);
        }

        prefix.push_str("## ");
        prefix.push_str(section.kind.as_label());
        prefix.push(':');
        prefix.push_str(name);
        prefix.push('\n');
        prefix.push_str(section.content.trim_end());
        prefix.push_str("\n\n");
    }

    Ok(prefix)
}

#[cfg(test)]
mod tests {
    use super::{compile_prompt, PromptCompileInput, PromptVersionInfo};
    use crate::{PromptSection, PromptSectionKind};

    fn stable_sections() -> Vec<PromptSection> {
        vec![
            PromptSection::new(PromptSectionKind::System, "kernel", "Be precise."),
            PromptSection::new(
                PromptSectionKind::Policy,
                "disk",
                "Avoid uncontrolled writes.",
            ),
        ]
    }

    #[test]
    fn same_stable_sections_with_different_user_request_keep_prefix_hash() {
        let version = PromptVersionInfo::new("0.1.0");
        let first = compile_prompt(PromptCompileInput::new(
            stable_sections(),
            "Analyze sample A.",
            version.clone(),
        ));
        let second = compile_prompt(PromptCompileInput::new(
            stable_sections(),
            "Analyze sample B.",
            version,
        ));

        match (first, second) {
            (Ok(first), Ok(second)) => assert_eq!(first.prefix_hash, second.prefix_hash),
            _ => panic!("prompt compilation should succeed for named sections"),
        }
    }

    #[test]
    fn changing_stable_section_changes_prefix_hash() {
        let version = PromptVersionInfo::new("0.1.0");
        let first = compile_prompt(PromptCompileInput::new(
            stable_sections(),
            "Analyze sample A.",
            version.clone(),
        ));
        let mut changed_sections = stable_sections();
        changed_sections[0].content = "Be concise.".to_owned();
        let second = compile_prompt(PromptCompileInput::new(
            changed_sections,
            "Analyze sample A.",
            version,
        ));

        match (first, second) {
            (Ok(first), Ok(second)) => assert_ne!(first.prefix_hash, second.prefix_hash),
            _ => panic!("prompt compilation should succeed for named sections"),
        }
    }
}
