#![forbid(unsafe_code)]
//! Minimal command handling for the `deepseek-science` binary.
//!
//! The CLI intentionally uses `std::env::args` in Phase 1 to avoid pulling in a
//! command-line framework before the command surface exists.

use std::fs;

use deepseek_science_artifacts::hash_bytes;
use deepseek_science_chemistry::{
    KineticsAnalysisResult, KineticsColumns, KineticsComparisonBasis, KineticsError,
    KineticsModelKind, KineticsReviewCheckKind, KineticsReviewSeverity, KineticsReviewStatus,
    ValidatedKineticsInput,
};
use deepseek_science_common::{mean, parse_simple_numeric_csv};
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
    /// Text written to stderr.
    pub stderr: String,
}

impl CliOutput {
    fn success(stdout: String) -> Self {
        Self {
            exit_code: 0,
            stdout,
            stderr: String::new(),
        }
    }

    fn user_error(message: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            stdout: String::new(),
            stderr: format!("error: {}\n", message.into()),
        }
    }

    fn user_error_with_usage(message: impl Into<String>, usage: &str) -> Self {
        Self {
            exit_code: 1,
            stdout: String::new(),
            stderr: format!("error: {}\n\n{usage}", message.into()),
        }
    }

    fn command_error(message: impl Into<String>) -> Self {
        Self {
            exit_code: 2,
            stdout: String::new(),
            stderr: format!("error: {}\n\n{}", message.into(), usage()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct KineticsAnalyzeArgs {
    input_path: String,
    time_column: String,
    concentration_column: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CliError {
    User(String),
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
        Some("doctor") => CliOutput::success(doctor_output()),
        Some("version") => {
            CliOutput::success(format!("deepseek-science {}\n", env!("CARGO_PKG_VERSION")))
        }
        Some("kinetics") => run_kinetics_command(args),
        Some("help") | Some("--help") | Some("-h") | None => CliOutput::success(usage()),
        Some(command) => CliOutput::command_error(format!("unknown command: {command}")),
    }
}

fn usage() -> String {
    "Usage: deepseek-science <doctor|version|help|kinetics>\n".to_owned()
}

fn kinetics_usage() -> &'static str {
    "Usage: deepseek-science kinetics <analyze>\n"
}

fn kinetics_analyze_usage() -> &'static str {
    "Usage: deepseek-science kinetics analyze --input <path> --time-column <column> --concentration-column <column>\n"
}

fn run_kinetics_command<I>(mut args: I) -> CliOutput
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        Some("analyze") => run_kinetics_analyze(args),
        Some(command) => CliOutput::user_error_with_usage(
            format!("unknown kinetics subcommand: {command}"),
            kinetics_usage(),
        ),
        None => CliOutput::user_error_with_usage("missing kinetics subcommand", kinetics_usage()),
    }
}

fn run_kinetics_analyze<I>(args: I) -> CliOutput
where
    I: IntoIterator<Item = String>,
{
    let args = match parse_kinetics_analyze_args(args) {
        Ok(args) => args,
        Err(CliError::User(message)) => {
            return CliOutput::user_error_with_usage(message, kinetics_analyze_usage());
        }
    };

    match analyze_kinetics_csv(&args) {
        Ok(output) => CliOutput::success(output),
        Err(CliError::User(message)) => CliOutput::user_error(message),
    }
}

fn parse_kinetics_analyze_args<I>(args: I) -> Result<KineticsAnalyzeArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut input_path = None;
    let mut time_column = None;
    let mut concentration_column = None;
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => {
                let value = next_option_value(&mut args, "--input")?;
                set_required_arg(&mut input_path, "--input", value)?;
            }
            "--time-column" => {
                let value = next_option_value(&mut args, "--time-column")?;
                set_required_arg(&mut time_column, "--time-column", value)?;
            }
            "--concentration-column" => {
                let value = next_option_value(&mut args, "--concentration-column")?;
                set_required_arg(&mut concentration_column, "--concentration-column", value)?;
            }
            value if value.starts_with("--") => {
                return Err(CliError::User(format!("unknown argument {value}")));
            }
            value => {
                return Err(CliError::User(format!(
                    "unexpected positional argument {value}"
                )));
            }
        }
    }

    Ok(KineticsAnalyzeArgs {
        input_path: input_path
            .ok_or_else(|| CliError::User("missing required argument --input".to_string()))?,
        time_column: time_column
            .ok_or_else(|| CliError::User("missing required argument --time-column".to_string()))?,
        concentration_column: concentration_column.ok_or_else(|| {
            CliError::User("missing required argument --concentration-column".to_string())
        })?,
    })
}

fn next_option_value<I>(args: &mut I, option_name: &str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    let Some(value) = args.next() else {
        return Err(CliError::User(format!("missing value for {option_name}")));
    };

    if value.is_empty() || value.starts_with("--") {
        return Err(CliError::User(format!("missing value for {option_name}")));
    }

    Ok(value)
}

fn set_required_arg(
    target: &mut Option<String>,
    option_name: &str,
    value: String,
) -> Result<(), CliError> {
    if target.is_some() {
        return Err(CliError::User(format!("duplicate argument {option_name}")));
    }

    *target = Some(value);
    Ok(())
}

fn analyze_kinetics_csv(args: &KineticsAnalyzeArgs) -> Result<String, CliError> {
    let csv_text = fs::read_to_string(&args.input_path).map_err(|error| {
        CliError::User(format!(
            "could not read input file `{}`: {error}",
            args.input_path
        ))
    })?;
    let table = parse_simple_numeric_csv(&csv_text)
        .map_err(|error| CliError::User(format!("invalid CSV: {error}")))?;
    let columns = KineticsColumns::new(&args.time_column, &args.concentration_column)
        .map_err(format_kinetics_error)?;
    let input =
        ValidatedKineticsInput::from_table(&table, &columns).map_err(format_kinetics_error)?;
    let analysis = KineticsAnalysisResult::analyze(&input).map_err(format_kinetics_error)?;

    Ok(format_kinetics_analysis_output(
        &args.input_path,
        &args.time_column,
        &args.concentration_column,
        &analysis,
    ))
}

fn format_kinetics_error(error: KineticsError) -> CliError {
    CliError::User(format!("kinetics analysis failed: {error}"))
}

fn format_kinetics_analysis_output(
    input_path: &str,
    time_column: &str,
    concentration_column: &str,
    analysis: &KineticsAnalysisResult,
) -> String {
    let findings = &analysis.review.findings;
    let mut output = format!(
        "\
DeepSeek_Science kinetics analyze
input: {input_path}
time_column: {time_column}
concentration_column: {concentration_column}
valid_points: {valid_points}
rejected_rows: {rejected_rows}
first_order.k: {first_order_k:.6}
first_order.r_squared: {first_order_r_squared:.6}
second_order.k: {second_order_k:.6}
second_order.r_squared: {second_order_r_squared:.6}
preferred_model: {preferred_model}
comparison_basis: {comparison_basis}
preferred_note: Preferred by MVP r_squared heuristic; not definitive model selection.
review_status: {review_status}
review_findings: {review_finding_count}
",
        valid_points = analysis.valid_point_count(),
        rejected_rows = analysis.rejected_row_count(),
        first_order_k = analysis.comparison.first_order.rate_constant_k,
        first_order_r_squared = analysis.comparison.first_order.r_squared,
        second_order_k = analysis.comparison.second_order.rate_constant_k,
        second_order_r_squared = analysis.comparison.second_order.r_squared,
        preferred_model = model_kind_label(analysis.preferred_model()),
        comparison_basis = comparison_basis_label(analysis.comparison_basis()),
        review_status = review_status_label(analysis.review_status()),
        review_finding_count = findings.len(),
    );

    if findings.is_empty() {
        output.push_str("review_finding_summary: none\n");
    } else {
        for (index, finding) in findings.iter().enumerate() {
            output.push_str(&format!(
                "review_finding.{index}: severity={severity}; check={check}; message={message}\n",
                severity = review_severity_label(finding.severity),
                check = review_check_kind_label(finding.check_kind),
                message = finding.message,
            ));
        }
    }

    output
}

fn model_kind_label(kind: KineticsModelKind) -> &'static str {
    match kind {
        KineticsModelKind::FirstOrder => "first_order",
        KineticsModelKind::SecondOrder => "second_order",
    }
}

fn comparison_basis_label(basis: KineticsComparisonBasis) -> &'static str {
    match basis {
        KineticsComparisonBasis::FiniteRSquaredMvpHeuristic => "finite_r_squared_mvp_heuristic",
    }
}

fn review_status_label(status: KineticsReviewStatus) -> &'static str {
    match status {
        KineticsReviewStatus::Passed => "passed",
        KineticsReviewStatus::PassedWithWarnings => "passed_with_warnings",
        KineticsReviewStatus::Failed => "failed",
    }
}

fn review_severity_label(severity: KineticsReviewSeverity) -> &'static str {
    match severity {
        KineticsReviewSeverity::Warning => "warning",
        KineticsReviewSeverity::Error => "error",
    }
}

fn review_check_kind_label(kind: KineticsReviewCheckKind) -> &'static str {
    match kind {
        KineticsReviewCheckKind::RateConstantMatchesSlope => "rate_constant_matches_slope",
        KineticsReviewCheckKind::FiniteMetrics => "finite_metrics",
        KineticsReviewCheckKind::RejectedRowsVisible => "rejected_rows_visible",
        KineticsReviewCheckKind::ComparisonBasisIsHeuristic => "comparison_basis_is_heuristic",
    }
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
    use deepseek_science_chemistry::{
        KineticsAnalysisResult, KineticsColumns, ValidatedKineticsInput,
    };
    use deepseek_science_common::{DataColumn, DataTable};

    use super::{
        format_kinetics_analysis_output, parse_kinetics_analyze_args, run_cli, CliError,
        KineticsAnalyzeArgs,
    };

    fn parse_args(args: &[&str]) -> Result<KineticsAnalyzeArgs, CliError> {
        parse_kinetics_analyze_args(args.iter().map(|value| (*value).to_string()))
    }

    fn numeric_column(name: &str, values: &[f64]) -> DataColumn {
        DataColumn::numeric(name, values.to_vec()).expect("test column should be valid")
    }

    fn analysis_result_with_rejected_row() -> KineticsAnalysisResult {
        let table = DataTable::new(vec![
            numeric_column("time_s", &[0.0, 99.0, 1.0]),
            numeric_column("concentration_mol_l", &[1.0, 0.0, (-0.25_f64).exp()]),
        ])
        .expect("test table should be valid");
        let columns = KineticsColumns::new("time_s", "concentration_mol_l")
            .expect("test columns should be valid");
        let input = ValidatedKineticsInput::from_table(&table, &columns)
            .expect("two positive rows should remain");

        KineticsAnalysisResult::analyze(&input).expect("test analysis should succeed")
    }

    #[test]
    fn version_command_prints_package_version() {
        let output = run_cli(["deepseek-science", "version"]);

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn unknown_command_returns_usage() {
        let output = run_cli(["deepseek-science", "unknown"]);

        assert_eq!(output.exit_code, 2);
        assert!(output.stderr.contains("Usage:"));
    }

    #[test]
    fn kinetics_analyze_arg_parser_accepts_valid_command() {
        let args = parse_args(&[
            "--input",
            "kinetics.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ])
        .expect("valid args should parse");

        assert_eq!(args.input_path, "kinetics.csv");
        assert_eq!(args.time_column, "time_s");
        assert_eq!(args.concentration_column, "concentration_mol_l");
    }

    #[test]
    fn kinetics_analyze_arg_parser_rejects_missing_input() {
        let result = parse_args(&[
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ]);

        assert_eq!(
            result,
            Err(CliError::User(
                "missing required argument --input".to_string()
            ))
        );
    }

    #[test]
    fn kinetics_analyze_arg_parser_rejects_missing_time_column() {
        let result = parse_args(&[
            "--input",
            "kinetics.csv",
            "--concentration-column",
            "concentration_mol_l",
        ]);

        assert_eq!(
            result,
            Err(CliError::User(
                "missing required argument --time-column".to_string()
            ))
        );
    }

    #[test]
    fn kinetics_analyze_arg_parser_rejects_missing_concentration_column() {
        let result = parse_args(&["--input", "kinetics.csv", "--time-column", "time_s"]);

        assert_eq!(
            result,
            Err(CliError::User(
                "missing required argument --concentration-column".to_string()
            ))
        );
    }

    #[test]
    fn kinetics_analyze_arg_parser_rejects_unknown_argument() {
        let result = parse_args(&[
            "--input",
            "kinetics.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--json",
        ]);

        assert_eq!(
            result,
            Err(CliError::User("unknown argument --json".to_string()))
        );
    }

    #[test]
    fn kinetics_analyze_arg_parser_rejects_duplicate_argument() {
        let result = parse_args(&[
            "--input",
            "kinetics.csv",
            "--input",
            "again.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ]);

        assert_eq!(
            result,
            Err(CliError::User("duplicate argument --input".to_string()))
        );
    }

    #[test]
    fn kinetics_analysis_format_includes_key_fields() {
        let analysis = analysis_result_with_rejected_row();
        let output = format_kinetics_analysis_output(
            "kinetics.csv",
            "time_s",
            "concentration_mol_l",
            &analysis,
        );

        assert!(output.contains("input: kinetics.csv"));
        assert!(output.contains("time_column: time_s"));
        assert!(output.contains("concentration_column: concentration_mol_l"));
        assert!(output.contains("valid_points: 2"));
        assert!(output.contains("rejected_rows: 1"));
        assert!(output.contains("first_order.k:"));
        assert!(output.contains("first_order.r_squared:"));
        assert!(output.contains("second_order.k:"));
        assert!(output.contains("second_order.r_squared:"));
        assert!(output.contains("preferred_model:"));
        assert!(output.contains("review_status: passed_with_warnings"));
        assert!(output.contains("review_findings: 1"));
    }

    #[test]
    fn kinetics_analysis_format_uses_cautious_mvp_wording() {
        let analysis = analysis_result_with_rejected_row();
        let output = format_kinetics_analysis_output(
            "kinetics.csv",
            "time_s",
            "concentration_mol_l",
            &analysis,
        );

        assert!(output.contains("Preferred by MVP r_squared heuristic"));
        assert!(!output.contains("definitive reaction order"));
        assert!(!output.contains("true model"));
        assert!(!output.contains("proved first-order"));
    }
}
