//! Stable hashing utilities for compiled prompt prefixes.

use crate::compiler::PromptVersionInfo;

/// Returns a lowercase BLAKE3 hash for stable prompt-prefix content.
pub fn hash_stable_prefix(prefix: &str) -> String {
    blake3::hash(prefix.as_bytes()).to_hex().to_string()
}

/// Returns a lowercase BLAKE3 hash for stable prompt cache identity.
///
/// The identity includes the stable prefix and stable version metadata. It
/// intentionally excludes the variable tail so per-run user requests do not
/// perturb provider cache keys.
pub fn hash_stable_prompt_identity(
    stable_prefix: &str,
    version_info: &PromptVersionInfo,
) -> String {
    let mut material = String::new();

    push_required_field(&mut material, "stable_prefix", stable_prefix);
    push_required_field(
        &mut material,
        "kernel_version",
        &version_info.kernel_version,
    );
    push_optional_field(
        &mut material,
        "domain_pack_version",
        version_info.domain_pack_version.as_deref(),
    );
    push_optional_field(
        &mut material,
        "tool_manifest_version",
        version_info.tool_manifest_version.as_deref(),
    );
    push_optional_field(
        &mut material,
        "project_context_version",
        version_info.project_context_version.as_deref(),
    );

    hash_stable_prefix(&material)
}

fn push_required_field(material: &mut String, name: &str, value: &str) {
    push_identity_field(material, name, "required", value);
}

fn push_optional_field(material: &mut String, name: &str, value: Option<&str>) {
    match value {
        Some(value) => push_identity_field(material, name, "some", value),
        None => push_identity_field(material, name, "none", ""),
    }
}

fn push_identity_field(material: &mut String, name: &str, state: &str, value: &str) {
    material.push_str(name);
    material.push('\n');
    material.push_str(state);
    material.push('\n');
    material.push_str(&value.len().to_string());
    material.push('\n');
    material.push_str(value);
    material.push('\n');
}
