//! Deterministic prompt prefix compilation.

use crate::{hash::hash_stable_prompt_identity, PromptError, PromptSection};
use serde::{Deserialize, Serialize};

/// Stable version metadata that contributes to prompt cache identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptVersionInfo {
    /// Stable prompt template or kernel contract version.
    pub kernel_version: String,
    /// Optional domain or workflow pack version bundle.
    pub domain_pack_version: Option<String>,
    /// Optional tool schema or manifest version.
    pub tool_manifest_version: Option<String>,
    /// Optional stable project context version or content hash.
    pub project_context_version: Option<String>,
}

impl PromptVersionInfo {
    /// Creates version metadata for the stable prompt template.
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
    /// BLAKE3 hash of `stable_prefix` and stable `version_info`.
    pub prefix_hash: String,
    /// Version metadata used to audit prompt provenance.
    pub version_info: PromptVersionInfo,
}

/// Compiles stable sections and a variable tail into a prompt.
pub fn compile_prompt(input: PromptCompileInput) -> Result<CompiledPrompt, PromptError> {
    validate_version_info(&input.version_info)?;

    let stable_prefix = compile_stable_prefix(&input.stable_sections)?;
    let variable_tail = format!("## user_request\n{}\n", input.user_request.trim_end());
    let full_prompt = format!("{stable_prefix}{variable_tail}");
    let prefix_hash = hash_stable_prompt_identity(&stable_prefix, &input.version_info);

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

fn validate_version_info(version_info: &PromptVersionInfo) -> Result<(), PromptError> {
    if version_info.kernel_version.trim().is_empty() {
        return Err(PromptError::EmptyTemplateVersion);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{compile_prompt, CompiledPrompt, PromptCompileInput, PromptVersionInfo};
    use crate::{PromptError, PromptSection, PromptSectionKind};

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

    fn compile(
        stable_sections: Vec<PromptSection>,
        user_request: &str,
        version_info: PromptVersionInfo,
    ) -> CompiledPrompt {
        compile_prompt(PromptCompileInput::new(
            stable_sections,
            user_request,
            version_info,
        ))
        .expect("prompt compilation should succeed for valid test input")
    }

    #[test]
    fn same_stable_sections_with_different_user_request_keep_prefix_hash() {
        let version = PromptVersionInfo::new("0.1.0");
        let first = compile(stable_sections(), "Analyze sample A.", version.clone());
        let second = compile(stable_sections(), "Analyze sample B.", version);

        assert_eq!(first.stable_prefix, second.stable_prefix);
        assert_ne!(first.variable_tail, second.variable_tail);
        assert_eq!(first.prefix_hash, second.prefix_hash);
    }

    #[test]
    fn changing_stable_section_changes_prefix_hash() {
        let version = PromptVersionInfo::new("0.1.0");
        let first = compile(stable_sections(), "Analyze sample A.", version.clone());
        let mut changed_sections = stable_sections();
        changed_sections[0].content = "Be concise.".to_owned();
        let second = compile(changed_sections, "Analyze sample A.", version);

        assert_ne!(first.prefix_hash, second.prefix_hash);
    }

    #[test]
    fn changing_stable_version_metadata_changes_prefix_hash() {
        let mut changed_version = PromptVersionInfo::new("0.1.0");
        changed_version.domain_pack_version = Some("chemistry-pack@2".to_owned());

        let first = compile(
            stable_sections(),
            "Analyze sample A.",
            PromptVersionInfo::new("0.1.0"),
        );
        let second = compile(stable_sections(), "Analyze sample A.", changed_version);

        assert_ne!(first.prefix_hash, second.prefix_hash);
    }

    #[test]
    fn changing_stable_section_order_changes_prefix_hash() {
        let version = PromptVersionInfo::new("0.1.0");
        let first = compile(stable_sections(), "Analyze sample A.", version.clone());
        let mut reordered_sections = stable_sections();
        reordered_sections.swap(0, 1);
        let second = compile(reordered_sections, "Analyze sample A.", version);

        assert_ne!(first.prefix_hash, second.prefix_hash);
    }

    #[test]
    fn full_prompt_contains_stable_prefix_and_variable_tail() {
        let compiled = compile(
            stable_sections(),
            "Analyze sample A.",
            PromptVersionInfo::new("0.1.0"),
        );

        assert!(compiled.full_prompt.contains(&compiled.stable_prefix));
        assert!(compiled.full_prompt.contains(&compiled.variable_tail));
        assert_eq!(
            compiled.full_prompt,
            format!("{}{}", compiled.stable_prefix, compiled.variable_tail)
        );
    }

    #[test]
    fn variable_tail_does_not_appear_in_stable_prefix() {
        let compiled = compile(
            stable_sections(),
            "unique variable request 7791",
            PromptVersionInfo::new("0.1.0"),
        );

        assert!(compiled
            .variable_tail
            .contains("unique variable request 7791"));
        assert!(!compiled
            .stable_prefix
            .contains("unique variable request 7791"));
    }

    #[test]
    fn compilation_is_deterministic_across_repeated_calls() {
        let input = PromptCompileInput::new(
            stable_sections(),
            "Analyze sample A.",
            PromptVersionInfo::new("0.1.0"),
        );

        let first = compile_prompt(input.clone());
        let second = compile_prompt(input);

        assert_eq!(first, second);
    }

    #[test]
    fn compilation_does_not_require_project_local_absolute_paths() {
        let mut version = PromptVersionInfo::new("0.1.0");
        version.project_context_version = Some("project-context-sha256:abc123".to_owned());

        let compiled = compile(stable_sections(), "Analyze sample A.", version);

        assert!(compiled.stable_prefix.contains("## system:kernel"));
        assert!(!compiled.stable_prefix.contains("/Users/"));
        assert!(!compiled.full_prompt.contains("/Users/"));
    }

    #[test]
    fn empty_template_version_is_rejected() {
        let compiled = compile_prompt(PromptCompileInput::new(
            stable_sections(),
            "Analyze sample A.",
            PromptVersionInfo::new(" "),
        ));

        assert_eq!(compiled, Err(PromptError::EmptyTemplateVersion));
    }
}
