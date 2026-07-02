#![forbid(unsafe_code)]
//! Minimal command handling for the `deepseek-science` binary.
//!
//! The CLI intentionally uses `std::env::args` in Phase 1 to avoid pulling in a
//! command-line framework before the command surface exists.

use deepseek_science_artifacts::hash_bytes;
use deepseek_science_common::mean;
use deepseek_science_core::ProjectId;
use deepseek_science_model::ModelCapabilities;
use deepseek_science_model_deepseek::DeepSeekModel;
use deepseek_science_prompt::PromptVersionInfo;
use deepseek_science_sandbox::SandboxPolicy;
use deepseek_science_storage::StorageLayout;
use deepseek_science_tools::ToolRegistry;

/// CLI command output and process status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliOutput {
    /// Process exit code.
    pub exit_code: i32,
    /// Text written to stdout.
    pub stdout: String,
}

/// Runs the CLI over an argument iterator including the binary name.
pub fn run_cli<I, S>(args: I) -> CliOutput
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let _binary_name = args.next();

    match args.next().as_deref() {
        Some("doctor") => CliOutput {
            exit_code: 0,
            stdout: doctor_output(),
        },
        Some("version") => CliOutput {
            exit_code: 0,
            stdout: format!("deepseek-science {}\n", env!("CARGO_PKG_VERSION")),
        },
        Some("help") | Some("--help") | Some("-h") | None => CliOutput {
            exit_code: 0,
            stdout: usage(),
        },
        Some(command) => CliOutput {
            exit_code: 2,
            stdout: format!("unknown command: {command}\n\n{}", usage()),
        },
    }
}

fn usage() -> String {
    "Usage: deepseek-science <doctor|version|help>\n".to_owned()
}

fn doctor_output() -> String {
    let project_id = ProjectId::new();
    let descriptor = DeepSeekModel::Reasoner.descriptor();
    let capabilities = ModelCapabilities::text_only(None);
    let version_info = PromptVersionInfo::new(env!("CARGO_PKG_VERSION"));
    let registry = ToolRegistry::new();
    let policy = SandboxPolicy::default();
    let layout = StorageLayout::for_project("workspace", project_id);
    let sample_mean = match mean(&[1.0, 2.0, 3.0]) {
        Ok(value) => value,
        Err(_) => 0.0,
    };
    let artifact_hash = hash_bytes(b"doctor");

    format!(
        "\
DeepSeek_Science doctor
version: {version}
phase: headless Rust kernel
core_project_id: {project_id}
default_model_provider: {provider}
default_model: {model}
text_capability_count: {capability_count}
prompt_kernel_version: {prompt_version}
registered_tools: {tool_count}
sandbox_network_allowed: {network_allowed}
storage_metadata_path: {metadata_path}
sample_mean: {sample_mean}
sample_artifact_hash_prefix: {hash_prefix}
status: ok
",
        version = env!("CARGO_PKG_VERSION"),
        provider = descriptor.provider,
        model = descriptor.model,
        capability_count = capabilities.modalities.len(),
        prompt_version = version_info.kernel_version,
        tool_count = registry.len(),
        network_allowed = policy.allow_network,
        metadata_path = layout.metadata_path.display(),
        hash_prefix = &artifact_hash[..8],
    )
}

#[cfg(test)]
mod tests {
    use super::run_cli;

    #[test]
    fn version_command_prints_package_version() {
        let output = run_cli(["deepseek-science", "version"]);

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn unknown_command_returns_usage() {
        let output = run_cli(["deepseek-science", "unknown"]);

        assert_eq!(output.exit_code, 2);
        assert!(output.stdout.contains("Usage:"));
    }
}
