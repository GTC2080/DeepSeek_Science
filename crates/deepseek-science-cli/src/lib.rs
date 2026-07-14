#![forbid(unsafe_code)]
//! Minimal command handling for the `deepseek-science` binary.
//!
//! The CLI intentionally uses `std::env::args` in Phase 1 to avoid pulling in a
//! command-line framework before the command surface exists.

use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use deepseek_science_artifacts::{hash_bytes, ArtifactError, UnregisteredArtifactEnvelope};
use deepseek_science_chemistry::{
    prepare_kinetics_artifact_envelope, render_kinetics_svg, KineticsAnalysisResult,
    KineticsColumns, KineticsComparisonBasis, KineticsError, KineticsModelKind, KineticsPlotData,
    KineticsReviewCheckKind, KineticsReviewSeverity, KineticsReviewStatus, ValidatedKineticsInput,
    CHEMISTRY_KINETICS_ARTIFACT_STEP, CHEMISTRY_KINETICS_CSV_WORKFLOW_ID,
    KINETICS_ANALYSIS_PAYLOAD_SCHEMA_VERSION, KINETICS_ARTIFACT_ENVELOPE_SCHEMA_VERSION,
    KINETICS_ARTIFACT_SOURCE_ROLE,
};
use deepseek_science_common::{
    assess_simple_csv_compatibility, inspect_delimited_text, inspect_text_encoding, mean,
    normalize_delimited_text, parse_simple_numeric_csv, BoundedLineEvidence, ByteOrderMark,
    DelimitedInspectionError, DelimitedTextInspection, DelimiterFinding, EncodingInspection,
    EncodingInspectionError, GenericTableShape, NormalizationError, SimpleCsvCompatibility,
    TableShapeReason, TextEncoding, MAX_INSPECTION_BYTES, MAX_NORMALIZED_OUTPUT_BYTES,
};
use deepseek_science_core::ProjectId;
use deepseek_science_model::ModelCapabilities;
use deepseek_science_model_deepseek::DeepSeekModel;
use deepseek_science_prompt::PromptVersionInfo;
use deepseek_science_sandbox::SandboxPolicy;
use deepseek_science_storage::{
    AtomicWritePlan, AtomicWriteRequest, StorageError, StorageLayout, StorageRoot, WriteMode,
};
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

    fn internal_error(message: impl Into<String>) -> Self {
        Self {
            exit_code: 2,
            stdout: String::new(),
            stderr: format!("error: {}\n", message.into()),
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
    json_output: bool,
    output_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct KineticsPlotArgs {
    input_path: String,
    time_column: String,
    concentration_column: String,
    output_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct KineticsArtifactArgs {
    input_path: String,
    time_column: String,
    concentration_column: String,
    output_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DataInspectArgs {
    input_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DataConvertArgs {
    input_path: String,
    output_path: String,
}

#[derive(Debug)]
enum BoundedReadError {
    Io(io::Error),
    LimitExceeded,
}

const MAX_DISPLAYED_HEADERS: usize = 32;
const MAX_HEADER_DISPLAY_CHARS: usize = 120;
const MAX_KINETICS_SVG_BYTES: usize = 4 * 1024 * 1024;
const MAX_KINETICS_ARTIFACT_INPUT_BYTES: usize = MAX_INSPECTION_BYTES;
const MAX_KINETICS_ARTIFACT_BYTES: usize = 4 * 1024 * 1024;
const KINETICS_ARTIFACT_PRODUCER_COMMAND: &str = "kinetics.artifact";
const KINETICS_SVG_ROOT_LINE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 960 640\" width=\"960\" height=\"640\" role=\"img\" aria-labelledby=\"plot-title plot-desc\" font-family=\"system-ui, sans-serif\">";

#[derive(Clone, Debug, Eq, PartialEq)]
enum CliError {
    User(String),
    Internal(String),
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
        Some("data") => run_data_command(args),
        Some("help") | Some("--help") | Some("-h") | None => CliOutput::success(usage()),
        Some(command) => CliOutput::command_error(format!("unknown command: {command}")),
    }
}

fn usage() -> String {
    "\
Usage: deepseek-science <doctor|version|help|kinetics|data>

Commands:
  kinetics analyze   Analyze one simple numeric kinetics CSV
  kinetics plot      Publish one deterministic kinetics SVG
  kinetics artifact  Publish one deterministic kinetics artifact envelope
  data inspect   Inspect one laboratory text file without modifying it
  data convert   Normalize one eligible laboratory text table into a new CSV
"
    .to_owned()
}

fn data_usage() -> &'static str {
    "\
Usage: deepseek-science data <inspect|convert>

Commands:
  inspect   Inspect one explicit laboratory text file without writing files
  convert   Normalize one eligible table into one new simple CSV file
"
}

fn data_inspect_usage() -> &'static str {
    "\
Usage:
  deepseek-science data inspect --input <path>

Options:
  --input <path>   Path to one regular laboratory text file
  -h, --help       Show this help

Limits and behavior:
  Input is limited to 16 MiB.
  Supported text is UTF-8, UTF-8 with BOM, or UTF-16LE/BE with a BOM.
  Only comma and tab delimiters are inspected.
  The command writes no files or hidden state.
  It does not modify, normalize, convert, or analyze the input.
  Incompatible table structure is reported rather than repaired.
"
}

fn data_convert_usage() -> &'static str {
    "\
Usage:
  deepseek-science data convert --input <path> --output <path>

Options:
  --input <path>    Path to one regular laboratory text file
  --output <path>   New simple CSV target; existing files are never overwritten
  -h, --help        Show this help

Limits and behavior:
  Input is limited to 16 MiB; normalized output is limited to 24 MiB.
  Eligible UTF-8 BOM, BOM-marked UTF-16LE/BE, and tab tables are normalized.
  Output is UTF-8 without BOM, comma-delimited, LF-only, with exactly one final LF.
  The output parent must already exist, and already-compatible input is rejected.
  Matrices, metadata, unit rows, blank rows, quotes, and whitespace repair are rejected.
  No scientific columns are inferred, no kinetics analysis runs, and no JSON mode exists.
"
}

fn kinetics_usage() -> &'static str {
    "\
Usage: deepseek-science kinetics <analyze|plot|artifact>

Commands:
  analyze   Analyze one simple numeric kinetics CSV
  plot      Publish one deterministic kinetics SVG
  artifact   Publish one deterministic provenance-bearing JSON envelope
"
}

fn kinetics_analyze_usage() -> &'static str {
    "\
Usage:
  deepseek-science kinetics analyze --input <path> --time-column <column> --concentration-column <column> [--json] [--output <path>]

Options:
  --input <path>                    Path to a simple numeric CSV file
  --time-column <column>            Time column name
  --concentration-column <column>   Concentration column name
  --json                            Write successful analysis as JSON to stdout
  --output <path>                   Save successful analysis as deterministic JSON
  -h, --help                        Show this help

Notes:
  Text output is the default. --json controls the stdout format.
  --output explicitly saves JSON after successful analysis.
  Existing targets are not overwritten, and parent directories are not created.
  Errors are written to stderr.
"
}

fn kinetics_plot_usage() -> &'static str {
    "\
Usage:
  deepseek-science kinetics plot \\
    --input <path> \\
    --time-column <column> \\
    --concentration-column <column> \\
    --output <path.svg>

Options:
  --input <path>                    Path to one regular simple numeric UTF-8 CSV file
  --time-column <column>            Exact time column name
  --concentration-column <column>   Exact concentration column name
  --output <path.svg>               New standalone deterministic SVG target
  -h, --help                        Show this help

Limits and behavior:
  Input is limited to 16 MiB.
  The output extension must be .svg (ASCII case-insensitive).
  The output parent must already exist; parent directories are not created.
  Existing targets are not overwritten.
  The command publishes no JSON sidecar or other persistent output.
"
}

fn kinetics_artifact_usage() -> &'static str {
    "\
Usage:
  deepseek-science kinetics artifact \\
    --input <path> \\
    --time-column <column> \\
    --concentration-column <column> \\
    --output <path.json>

Options:
  --input <path>                    Path to one regular simple numeric UTF-8 CSV file
  --time-column <column>            Exact time column name
  --concentration-column <column>   Exact concentration column name
  --output <path.json>              New deterministic JSON envelope target
  -h, --help                        Show this help

Limits and behavior:
  Input is limited to 16 MiB and must be UTF-8 without a BOM.
  The output extension must be .json (ASCII case-insensitive).
  The output parent must already exist; parent directories are not created.
  Existing targets are not overwritten.
  The command publishes one single envelope and no payload sidecar, manifest sidecar, SVG, project, or run record.
  Errors are written to stderr.
"
}

fn run_data_command<I>(args: I) -> CliOutput
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    match args.as_slice() {
        [flag] if flag == "--help" || flag == "-h" => CliOutput::success(data_usage().to_string()),
        [subcommand, remaining @ ..] if subcommand == "inspect" => {
            run_data_inspect(remaining.iter().cloned())
        }
        [subcommand, remaining @ ..] if subcommand == "convert" => {
            run_data_convert(remaining.iter().cloned())
        }
        [command, ..] => CliOutput::user_error_with_usage(
            format!("unknown data subcommand: {command}"),
            data_usage(),
        ),
        [] => CliOutput::user_error_with_usage("missing data subcommand", data_usage()),
    }
}

fn run_data_convert<I>(args: I) -> CliOutput
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if matches!(args.as_slice(), [flag] if flag == "--help" || flag == "-h") {
        return CliOutput::success(data_convert_usage().to_string());
    }

    let args = match parse_data_convert_args(args) {
        Ok(args) => args,
        Err(CliError::User(message)) => {
            return CliOutput::user_error_with_usage(message, data_convert_usage());
        }
        Err(CliError::Internal(message)) => return CliOutput::internal_error(message),
    };

    match convert_data_file(&args) {
        Ok(output) => CliOutput::success(output),
        Err(CliError::User(message)) => CliOutput::user_error(message),
        Err(CliError::Internal(message)) => CliOutput::internal_error(message),
    }
}

fn run_data_inspect<I>(args: I) -> CliOutput
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if matches!(args.as_slice(), [flag] if flag == "--help" || flag == "-h") {
        return CliOutput::success(data_inspect_usage().to_string());
    }

    let args = match parse_data_inspect_args(args) {
        Ok(args) => args,
        Err(CliError::User(message)) => {
            return CliOutput::user_error_with_usage(message, data_inspect_usage());
        }
        Err(CliError::Internal(message)) => return CliOutput::internal_error(message),
    };

    match inspect_data_file(&args.input_path) {
        Ok(output) => CliOutput::success(output),
        Err(CliError::User(message)) => CliOutput::user_error(message),
        Err(CliError::Internal(message)) => CliOutput::internal_error(message),
    }
}

fn parse_data_inspect_args<I>(args: I) -> Result<DataInspectArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut input_path = None;
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => {
                let value = next_data_option_value(&mut args, "--input")?;
                set_required_arg(&mut input_path, "--input", value)?;
            }
            value if value.starts_with('-') => {
                return Err(CliError::User(format!("unknown argument {value}")));
            }
            value => {
                return Err(CliError::User(format!(
                    "unexpected positional argument {value}"
                )));
            }
        }
    }

    Ok(DataInspectArgs {
        input_path: input_path
            .ok_or_else(|| CliError::User("missing required argument --input".to_string()))?,
    })
}

fn parse_data_convert_args<I>(args: I) -> Result<DataConvertArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut input_path = None;
    let mut output_path = None;
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => {
                let value = next_data_option_value(&mut args, "--input")?;
                set_required_arg(&mut input_path, "--input", value)?;
            }
            "--output" => {
                let value = next_data_option_value(&mut args, "--output")?;
                set_required_arg(&mut output_path, "--output", value)?;
            }
            value if value.starts_with('-') => {
                return Err(CliError::User(format!("unknown argument {value}")));
            }
            value => {
                return Err(CliError::User(format!(
                    "unexpected positional argument {value}"
                )));
            }
        }
    }

    Ok(DataConvertArgs {
        input_path: input_path
            .ok_or_else(|| CliError::User("missing required argument --input".to_string()))?,
        output_path: output_path
            .ok_or_else(|| CliError::User("missing required argument --output".to_string()))?,
    })
}

fn next_data_option_value<I>(args: &mut I, option_name: &str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    let Some(value) = args.next() else {
        return Err(CliError::User(format!("missing value for {option_name}")));
    };

    if value.is_empty() || value.starts_with('-') {
        return Err(CliError::User(format!("missing value for {option_name}")));
    }

    Ok(value)
}

fn run_kinetics_command<I>(mut args: I) -> CliOutput
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        Some("analyze") => run_kinetics_analyze(args),
        Some("plot") => run_kinetics_plot(args),
        Some("artifact") => run_kinetics_artifact(args),
        Some(command) => CliOutput::user_error_with_usage(
            format!("unknown kinetics subcommand: {command}"),
            kinetics_usage(),
        ),
        None => CliOutput::user_error_with_usage("missing kinetics subcommand", kinetics_usage()),
    }
}

fn run_kinetics_artifact<I>(args: I) -> CliOutput
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if matches!(args.as_slice(), [flag] if flag == "--help" || flag == "-h") {
        return CliOutput::success(kinetics_artifact_usage().to_string());
    }

    let args = match parse_kinetics_artifact_args(args) {
        Ok(args) => args,
        Err(CliError::User(message)) => {
            return CliOutput::user_error_with_usage(message, kinetics_artifact_usage());
        }
        Err(CliError::Internal(message)) => return CliOutput::internal_error(message),
    };

    match create_kinetics_artifact(&args) {
        Ok(output) => CliOutput::success(output),
        Err(CliError::User(message)) => CliOutput::user_error(message),
        Err(CliError::Internal(message)) => CliOutput::internal_error(message),
    }
}

fn run_kinetics_plot<I>(args: I) -> CliOutput
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if matches!(args.as_slice(), [flag] if flag == "--help" || flag == "-h") {
        return CliOutput::success(kinetics_plot_usage().to_string());
    }

    let args = match parse_kinetics_plot_args(args) {
        Ok(args) => args,
        Err(CliError::User(message)) => {
            return CliOutput::user_error_with_usage(message, kinetics_plot_usage());
        }
        Err(CliError::Internal(message)) => return CliOutput::internal_error(message),
    };

    match plot_kinetics_csv(&args) {
        Ok(output) => CliOutput::success(output),
        Err(CliError::User(message)) => CliOutput::user_error(message),
        Err(CliError::Internal(message)) => CliOutput::internal_error(message),
    }
}

fn run_kinetics_analyze<I>(args: I) -> CliOutput
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if is_kinetics_analyze_help(&args) {
        return CliOutput::success(kinetics_analyze_usage().to_string());
    }

    let args = match parse_kinetics_analyze_args(args) {
        Ok(args) => args,
        Err(CliError::User(message)) => {
            return CliOutput::user_error_with_usage(message, kinetics_analyze_usage());
        }
        Err(CliError::Internal(message)) => return CliOutput::internal_error(message),
    };

    match analyze_kinetics_csv(&args) {
        Ok(output) => CliOutput::success(output),
        Err(CliError::User(message)) => CliOutput::user_error(message),
        Err(CliError::Internal(message)) => CliOutput::internal_error(message),
    }
}

fn is_kinetics_analyze_help(args: &[String]) -> bool {
    matches!(args, [flag] if flag == "--help" || flag == "-h")
}

fn parse_kinetics_analyze_args<I>(args: I) -> Result<KineticsAnalyzeArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut input_path = None;
    let mut time_column = None;
    let mut concentration_column = None;
    let mut json_output = false;
    let mut output_path = None;
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
            "--json" => {
                set_flag(&mut json_output, "--json")?;
            }
            "--output" => {
                let value = next_option_value(&mut args, "--output")?;
                set_required_arg(&mut output_path, "--output", value)?;
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
        json_output,
        output_path,
    })
}

fn parse_kinetics_artifact_args<I>(args: I) -> Result<KineticsArtifactArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut input_path = None;
    let mut time_column = None;
    let mut concentration_column = None;
    let mut output_path = None;
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => {
                let value = next_data_option_value(&mut args, "--input")?;
                set_required_arg(&mut input_path, "--input", value)?;
            }
            "--time-column" => {
                let value = next_data_option_value(&mut args, "--time-column")?;
                set_required_arg(&mut time_column, "--time-column", value)?;
            }
            "--concentration-column" => {
                let value = next_data_option_value(&mut args, "--concentration-column")?;
                set_required_arg(&mut concentration_column, "--concentration-column", value)?;
            }
            "--output" => {
                let value = next_data_option_value(&mut args, "--output")?;
                set_required_arg(&mut output_path, "--output", value)?;
            }
            value if value.starts_with('-') => {
                return Err(CliError::User(format!("unknown argument {value}")));
            }
            value => {
                return Err(CliError::User(format!(
                    "unexpected positional argument {value}"
                )));
            }
        }
    }

    Ok(KineticsArtifactArgs {
        input_path: input_path
            .ok_or_else(|| CliError::User("missing required argument --input".to_string()))?,
        time_column: time_column
            .ok_or_else(|| CliError::User("missing required argument --time-column".to_string()))?,
        concentration_column: concentration_column.ok_or_else(|| {
            CliError::User("missing required argument --concentration-column".to_string())
        })?,
        output_path: output_path
            .ok_or_else(|| CliError::User("missing required argument --output".to_string()))?,
    })
}

fn parse_kinetics_plot_args<I>(args: I) -> Result<KineticsPlotArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut input_path = None;
    let mut time_column = None;
    let mut concentration_column = None;
    let mut output_path = None;
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => {
                let value = next_data_option_value(&mut args, "--input")?;
                set_required_arg(&mut input_path, "--input", value)?;
            }
            "--time-column" => {
                let value = next_data_option_value(&mut args, "--time-column")?;
                set_required_arg(&mut time_column, "--time-column", value)?;
            }
            "--concentration-column" => {
                let value = next_data_option_value(&mut args, "--concentration-column")?;
                set_required_arg(&mut concentration_column, "--concentration-column", value)?;
            }
            "--output" => {
                let value = next_data_option_value(&mut args, "--output")?;
                set_required_arg(&mut output_path, "--output", value)?;
            }
            value if value.starts_with('-') => {
                return Err(CliError::User(format!("unknown argument {value}")));
            }
            value => {
                return Err(CliError::User(format!(
                    "unexpected positional argument {value}"
                )));
            }
        }
    }

    Ok(KineticsPlotArgs {
        input_path: input_path
            .ok_or_else(|| CliError::User("missing required argument --input".to_string()))?,
        time_column: time_column
            .ok_or_else(|| CliError::User("missing required argument --time-column".to_string()))?,
        concentration_column: concentration_column.ok_or_else(|| {
            CliError::User("missing required argument --concentration-column".to_string())
        })?,
        output_path: output_path
            .ok_or_else(|| CliError::User("missing required argument --output".to_string()))?,
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

fn set_flag(target: &mut bool, option_name: &str) -> Result<(), CliError> {
    if *target {
        return Err(CliError::User(format!("duplicate argument {option_name}")));
    }

    *target = true;
    Ok(())
}

fn inspect_data_file(input_path: &str) -> Result<String, CliError> {
    let bytes = read_inspection_input(input_path)?;
    let encoding = inspect_text_encoding(&bytes).map_err(format_encoding_inspection_error)?;
    let table =
        inspect_delimited_text(&encoding.text).map_err(format_delimited_inspection_error)?;
    let compatibility = assess_simple_csv_compatibility(&encoding, &table);

    Ok(format_data_inspection_report(
        &encoding,
        &table,
        compatibility,
    ))
}

fn convert_data_file(args: &DataConvertArgs) -> Result<String, CliError> {
    if paths_are_lexically_equal(&args.input_path, &args.output_path) {
        return Err(CliError::User(
            "input and output paths must be different".to_string(),
        ));
    }

    let bytes = read_inspection_input(&args.input_path)?;
    let encoding = inspect_text_encoding(&bytes).map_err(format_encoding_inspection_error)?;
    let table =
        inspect_delimited_text(&encoding.text).map_err(format_delimited_inspection_error)?;
    let normalized =
        normalize_delimited_text(&encoding, &table).map_err(format_normalization_error)?;

    parse_simple_numeric_csv(&normalized).map_err(|_| {
        CliError::Internal("normalized output failed simple CSV validation".to_string())
    })?;
    if normalized.len() > MAX_NORMALIZED_OUTPUT_BYTES {
        return Err(CliError::Internal(
            "normalized output exceeded its validated size limit".to_string(),
        ));
    }

    write_converted_output_file(&args.output_path, normalized.as_bytes())?;
    format_data_conversion_report(&encoding, &table, normalized.len())
}

fn paths_are_lexically_equal(input_path: &str, output_path: &str) -> bool {
    Path::new(input_path) == Path::new(output_path)
}

fn read_inspection_input(input_path: &str) -> Result<Vec<u8>, CliError> {
    let file = fs::File::open(input_path).map_err(|error| {
        CliError::User(format!("could not open input file `{input_path}`: {error}"))
    })?;
    let metadata = file.metadata().map_err(|error| {
        CliError::User(format!(
            "could not inspect input file `{input_path}`: {error}"
        ))
    })?;
    if !metadata.is_file() {
        return Err(CliError::User(format!(
            "input path `{input_path}` must refer to a regular file"
        )));
    }
    if metadata.len() > MAX_INSPECTION_BYTES as u64 {
        return Err(CliError::User(format!(
            "input file `{input_path}` exceeds the fixed 16 MiB inspection limit"
        )));
    }

    let bytes = match read_bounded(file, MAX_INSPECTION_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedReadError::LimitExceeded) => {
            return Err(CliError::User(format!(
                "input file `{input_path}` exceeds the fixed 16 MiB inspection limit"
            )));
        }
        Err(BoundedReadError::Io(error)) => {
            return Err(CliError::User(format!(
                "could not read input file `{input_path}`: {error}"
            )));
        }
    };

    Ok(bytes)
}

fn read_kinetics_artifact_input(input_path: &str) -> Result<Vec<u8>, CliError> {
    let file = fs::File::open(input_path).map_err(|error| {
        CliError::User(format!(
            "could not open input file `{input_path}` for kinetics artifact: {error}"
        ))
    })?;
    let metadata = file.metadata().map_err(|error| {
        CliError::User(format!(
            "could not inspect kinetics artifact input file `{input_path}`: {error}"
        ))
    })?;
    validate_kinetics_artifact_input_metadata(
        input_path,
        metadata.is_file(),
        metadata.len(),
        MAX_KINETICS_ARTIFACT_INPUT_BYTES,
    )?;

    read_kinetics_artifact_bounded(file, input_path, MAX_KINETICS_ARTIFACT_INPUT_BYTES)
}

fn validate_kinetics_artifact_input_metadata(
    input_path: &str,
    is_regular_file: bool,
    actual_bytes: u64,
    maximum: usize,
) -> Result<(), CliError> {
    if !is_regular_file {
        return Err(CliError::User(format!(
            "kinetics artifact input path `{input_path}` must refer to a regular file"
        )));
    }
    let maximum = u64::try_from(maximum).map_err(|_| {
        CliError::Internal("kinetics artifact input limit exceeds the supported range".to_string())
    })?;
    if actual_bytes > maximum {
        return Err(kinetics_artifact_input_limit_error(input_path));
    }
    Ok(())
}

fn read_kinetics_artifact_bounded<R: Read>(
    reader: R,
    input_path: &str,
    maximum: usize,
) -> Result<Vec<u8>, CliError> {
    match read_bounded(reader, maximum) {
        Ok(bytes) => Ok(bytes),
        Err(BoundedReadError::LimitExceeded) => {
            Err(kinetics_artifact_input_limit_error(input_path))
        }
        Err(BoundedReadError::Io(error)) => Err(CliError::User(format!(
            "could not read kinetics artifact input file `{input_path}`: {error}"
        ))),
    }
}

fn kinetics_artifact_input_limit_error(input_path: &str) -> CliError {
    CliError::User(format!(
        "kinetics artifact input file `{input_path}` exceeds the fixed 16 MiB limit"
    ))
}

fn decode_kinetics_artifact_input(bytes: &[u8]) -> Result<&str, CliError> {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(CliError::User(
            "kinetics artifact input must be UTF-8 without a BOM".to_string(),
        ));
    }
    std::str::from_utf8(bytes).map_err(|error| {
        CliError::User(format!(
            "kinetics artifact input is not valid UTF-8 at byte offset {}",
            error.valid_up_to()
        ))
    })
}

fn read_bounded<R: Read>(reader: R, maximum: usize) -> Result<Vec<u8>, BoundedReadError> {
    let mut bytes = Vec::new();
    let mut limited = reader.take((maximum as u64).saturating_add(1));
    limited
        .read_to_end(&mut bytes)
        .map_err(BoundedReadError::Io)?;

    if bytes.len() > maximum {
        Err(BoundedReadError::LimitExceeded)
    } else {
        Ok(bytes)
    }
}

fn format_encoding_inspection_error(error: EncodingInspectionError) -> CliError {
    let message = match error {
        EncodingInspectionError::InspectionLimitExceeded { actual, maximum } => format!(
            "input has {actual} bytes, exceeding the fixed 16 MiB inspection limit of {maximum} bytes"
        ),
        EncodingInspectionError::UnsupportedBinaryInput { byte_offset } => format!(
            "unsupported binary or NUL input at byte offset {byte_offset}"
        ),
        EncodingInspectionError::UnsupportedOrAmbiguousEncoding { byte_offset } => format!(
            "unsupported or ambiguous encoding at byte offset {byte_offset}; BOM-free UTF-16 is not detected"
        ),
        EncodingInspectionError::InvalidUtf8 { byte_offset } => {
            format!("invalid UTF-8 at original-input byte offset {byte_offset}")
        }
        EncodingInspectionError::InvalidUtf16 { byte_offset } => {
            format!("invalid UTF-16 at original-input byte offset {byte_offset}")
        }
    };

    CliError::User(message)
}

fn format_delimited_inspection_error(error: DelimitedInspectionError) -> CliError {
    match error {
        DelimitedInspectionError::InspectionLimitExceeded { actual, maximum } => CliError::User(
            format!(
                "decoded text has {actual} bytes, exceeding the fixed 16 MiB inspection limit of {maximum} bytes"
            ),
        ),
    }
}

fn format_normalization_error(error: NormalizationError) -> CliError {
    match error {
        NormalizationError::AlreadyCompatible
        | NormalizationError::StructuralConversionIneligible
        | NormalizationError::UnsafeCellContent { .. }
        | NormalizationError::OutputLimitExceeded { .. } => CliError::User(error.to_string()),
        NormalizationError::ArithmeticOverflow | NormalizationError::InspectionInvariant => {
            CliError::Internal(error.to_string())
        }
    }
}

fn format_data_conversion_report(
    encoding: &EncodingInspection,
    table: &DelimitedTextInspection,
    output_bytes: usize,
) -> Result<String, CliError> {
    let region = table.region.as_ref().ok_or_else(|| {
        CliError::Internal("conversion table region was not available".to_string())
    })?;

    Ok(format!(
        "\
conversion_status: complete
source_encoding: {source_encoding}
source_bom: {source_bom}
source_delimiter: {source_delimiter}
output_encoding: utf-8
output_bom: none
output_delimiter: comma
line_endings: lf
field_count: {field_count}
data_rows: {data_rows}
input_bytes: {input_bytes}
output_bytes: {output_bytes}
",
        source_encoding = text_encoding_label(encoding.encoding),
        source_bom = bom_label(encoding.bom),
        source_delimiter = delimiter_label(table.delimiter),
        field_count = region.stable_field_count,
        data_rows = region.fully_numeric_row_count,
        input_bytes = encoding.original_byte_len,
    ))
}

fn format_data_inspection_report(
    encoding: &EncodingInspection,
    table: &DelimitedTextInspection,
    compatibility: SimpleCsvCompatibility,
) -> String {
    let mut output = format!(
        "\
inspection_status: {inspection_status}
encoding: {encoding}
bom: {bom}
input_bytes: {input_bytes}
delimiter: {delimiter}
physical_lines: {physical_lines}
blank_lines: {blank_lines}
nonblank_lines: {nonblank_lines}
",
        inspection_status = if table.complete {
            "complete"
        } else {
            "partial"
        },
        encoding = text_encoding_label(encoding.encoding),
        bom = bom_label(encoding.bom),
        input_bytes = encoding.original_byte_len,
        delimiter = delimiter_label(table.delimiter),
        physical_lines = table.physical_line_count,
        blank_lines = table.blank_line_count,
        nonblank_lines = table.nonblank_line_count,
    );

    if let Some(region) = table.region.as_ref() {
        output.push_str(&format!(
            "\
table_region: {first}-{last}
field_count: {field_count}
header_lines: {header_lines}
",
            first = region.first_line,
            last = region.last_line,
            field_count = region.stable_field_count,
            header_lines = format_line_evidence(&region.header_candidate_lines),
        ));
        append_headers(&mut output, &region.header_labels);
        output.push_str(&format!(
            "\
fully_numeric_rows: {fully_numeric_rows}
nonnumeric_rows: {nonnumeric_rows}
nonnumeric_lines: {nonnumeric_lines}
metadata_lines: {metadata_lines}
inconsistent_width_lines: {inconsistent_width_lines}
additional_content_lines: {additional_content_lines}
empty_cell_lines: {empty_cell_lines}
non_finite_numeric_lines: {non_finite_numeric_lines}
",
            fully_numeric_rows = region.fully_numeric_row_count,
            nonnumeric_rows = region.nonnumeric_row_count,
            nonnumeric_lines = format_line_evidence(&region.nonnumeric_lines),
            metadata_lines = format_line_evidence(&region.metadata_lines),
            inconsistent_width_lines = format_line_evidence(&region.inconsistent_width_lines),
            additional_content_lines = format_line_evidence(&region.additional_content_lines),
            empty_cell_lines = format_line_evidence(&region.empty_numeric_cell_lines),
            non_finite_numeric_lines = format_line_evidence(&region.non_finite_numeric_lines),
        ));
    } else {
        output.push_str(
            "\
table_region: none
field_count: none
header_lines: none
headers: none
fully_numeric_rows: none
nonnumeric_rows: none
nonnumeric_lines: none
metadata_lines: none
inconsistent_width_lines: none
additional_content_lines: none
empty_cell_lines: none
non_finite_numeric_lines: none
",
        );
    }

    output.push_str(&format!(
        "\
quoted_lines: {quoted_lines}
shape: {shape}
simple_csv_compatibility: {compatibility}
current_kinetics_workflow: {kinetics_compatibility}
",
        quoted_lines = format_line_evidence(&table.quoted_lines),
        shape = table_shape_label(table.shape),
        compatibility = simple_csv_compatibility_label(compatibility),
        kinetics_compatibility = kinetics_compatibility_label(compatibility),
    ));
    append_findings(&mut output, table, compatibility);

    output
}

fn append_headers(output: &mut String, headers: &[String]) {
    if headers.is_empty() {
        output.push_str("headers: none\n");
        return;
    }

    output.push_str("headers:\n");
    for header in headers.iter().take(MAX_DISPLAYED_HEADERS) {
        output.push_str("  - ");
        output.push_str(&sanitize_header_label(header));
        output.push('\n');
    }
    if headers.len() > MAX_DISPLAYED_HEADERS {
        output.push_str(&format!(
            "  - ... [{} additional headers omitted]\n",
            headers.len() - MAX_DISPLAYED_HEADERS
        ));
    }
}

fn sanitize_header_label(label: &str) -> String {
    let mut output = String::new();
    let mut characters = label.chars();

    for _ in 0..MAX_HEADER_DISPLAY_CHARS {
        let Some(character) = characters.next() else {
            return output;
        };
        match character {
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\\' => output.push_str("\\\\"),
            value if value.is_control() || matches!(value, '\u{2028}' | '\u{2029}') => {
                output.push_str(&format!("\\u{{{:x}}}", value as u32));
            }
            value => output.push(value),
        }
    }

    if characters.next().is_some() {
        output.push_str("… [truncated]");
    }
    output
}

fn format_line_evidence(evidence: &BoundedLineEvidence) -> String {
    if evidence.total_count == 0 {
        return "none".to_string();
    }

    let examples = evidence
        .example_lines
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    if evidence.additional_examples_omitted {
        format!(
            "{examples} (total: {}; additional omitted)",
            evidence.total_count
        )
    } else {
        examples
    }
}

fn append_findings(
    output: &mut String,
    table: &DelimitedTextInspection,
    compatibility: SimpleCsvCompatibility,
) {
    let mut findings = table
        .reasons
        .iter()
        .filter_map(|reason| table_reason_message(*reason))
        .collect::<Vec<_>>();
    if compatibility == SimpleCsvCompatibility::RequiresExplicitNormalization {
        findings.push(
            "explicit normalization is required; data convert accepts only eligible narrow tables",
        );
    }

    if findings.is_empty() {
        output.push_str("findings: none\n");
    } else {
        output.push_str("findings:\n");
        for finding in findings {
            output.push_str("  - ");
            output.push_str(finding);
            output.push('\n');
        }
    }
}

fn text_encoding_label(encoding: TextEncoding) -> &'static str {
    match encoding {
        TextEncoding::Utf8 => "utf-8",
        TextEncoding::Utf16Le => "utf-16le",
        TextEncoding::Utf16Be => "utf-16be",
    }
}

fn bom_label(bom: ByteOrderMark) -> &'static str {
    match bom {
        ByteOrderMark::None => "none",
        ByteOrderMark::Utf8 => "utf-8",
        ByteOrderMark::Utf16Le => "utf-16le",
        ByteOrderMark::Utf16Be => "utf-16be",
    }
}

fn delimiter_label(delimiter: DelimiterFinding) -> &'static str {
    match delimiter {
        DelimiterFinding::Comma => "comma",
        DelimiterFinding::Tab => "tab",
        DelimiterFinding::Ambiguous => "ambiguous",
        DelimiterFinding::Unsupported => "unsupported",
    }
}

fn table_shape_label(shape: GenericTableShape) -> &'static str {
    match shape {
        GenericTableShape::NumericNarrowTable => "numeric-narrow-table",
        GenericTableShape::NumericMatrix => "numeric-matrix",
        GenericTableShape::MixedOrUnsupported => "mixed-or-unsupported",
        GenericTableShape::Empty => "empty",
    }
}

fn simple_csv_compatibility_label(compatibility: SimpleCsvCompatibility) -> &'static str {
    match compatibility {
        SimpleCsvCompatibility::CompatibleAsIs => "compatible-as-is",
        SimpleCsvCompatibility::RequiresExplicitNormalization => "requires-explicit-normalization",
        SimpleCsvCompatibility::Incompatible => "incompatible",
    }
}

fn kinetics_compatibility_label(compatibility: SimpleCsvCompatibility) -> &'static str {
    match compatibility {
        SimpleCsvCompatibility::CompatibleAsIs => {
            "potentially-compatible-after-explicit-column-selection"
        }
        SimpleCsvCompatibility::RequiresExplicitNormalization => {
            "requires-normalization-before-analysis"
        }
        SimpleCsvCompatibility::Incompatible => "incompatible",
    }
}

fn table_reason_message(reason: TableShapeReason) -> Option<&'static str> {
    match reason {
        TableShapeReason::NoUsableTable => Some("no usable table rows were found"),
        TableShapeReason::QuotedInput => Some("quoted or multiline field parsing is unsupported"),
        TableShapeReason::UnsupportedDelimiter => {
            Some("no stable comma or tab table region was found")
        }
        TableShapeReason::AmbiguousDelimiter => {
            Some("comma and tab table regions are equally plausible")
        }
        TableShapeReason::AmbiguousTableRegion => {
            Some("multiple table regions are equally plausible")
        }
        TableShapeReason::MissingNamedHeader => Some("a named header row was not established"),
        TableShapeReason::MultipleHeaderRows => {
            Some("multiple header or unit rows require an explicit decision")
        }
        TableShapeReason::EmptyHeaderLabel => Some("a header label is empty"),
        TableShapeReason::DuplicateHeaderLabel => Some("a header label is duplicated"),
        TableShapeReason::NonnumericBody => Some("nonnumeric rows appear inside the numeric body"),
        TableShapeReason::InconsistentFieldCount => Some("inconsistent field counts were observed"),
        TableShapeReason::AdditionalTableContent => {
            Some("additional nonblank content follows the selected table region")
        }
        TableShapeReason::EmptyNumericCell => Some("an empty cell was observed"),
        TableShapeReason::NonFiniteNumericCell => Some("a non-finite numeric cell was observed"),
        TableShapeReason::MetadataBeforeTable => {
            Some("metadata appears before the selected table region")
        }
        TableShapeReason::NamedFiniteRectangle => None,
        TableShapeReason::MonotonicSiblingMatrix => {
            Some("numeric matrix structure is not accepted by the current kinetics workflow")
        }
    }
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

    let json_output = if args.json_output || args.output_path.is_some() {
        Some(format_kinetics_analysis_json_output(
            &args.input_path,
            &args.time_column,
            &args.concentration_column,
            &analysis,
        )?)
    } else {
        None
    };

    if let Some(output_path) = args.output_path.as_deref() {
        let bytes = json_output
            .as_deref()
            .ok_or_else(|| CliError::Internal("JSON output was not prepared".to_string()))?
            .as_bytes();
        write_json_output_file(output_path, bytes)?;
    }

    if args.json_output {
        json_output.ok_or_else(|| CliError::Internal("JSON output was not prepared".to_string()))
    } else {
        Ok(format_kinetics_analysis_output(
            &args.input_path,
            &args.time_column,
            &args.concentration_column,
            &analysis,
        ))
    }
}

fn create_kinetics_artifact(args: &KineticsArtifactArgs) -> Result<String, CliError> {
    if paths_are_lexically_equal(&args.input_path, &args.output_path) {
        return Err(CliError::User(
            "input and output paths must be different".to_string(),
        ));
    }
    validate_kinetics_artifact_output_path(&args.output_path)?;

    let raw_source_bytes = read_kinetics_artifact_input(&args.input_path)?;
    let csv_text = decode_kinetics_artifact_input(&raw_source_bytes)?;
    let table = parse_simple_numeric_csv(csv_text)
        .map_err(|error| CliError::User(format!("invalid CSV: {error}")))?;
    let columns = KineticsColumns::new(&args.time_column, &args.concentration_column)
        .map_err(format_kinetics_error)?;
    let input =
        ValidatedKineticsInput::from_table(&table, &columns).map_err(format_kinetics_error)?;
    let analysis = KineticsAnalysisResult::analyze(&input).map_err(format_kinetics_error)?;
    let payload_utf8 = format_kinetics_analysis_json_output(
        &args.input_path,
        &args.time_column,
        &args.concentration_column,
        &analysis,
    )?;
    let envelope = prepare_kinetics_artifact_envelope(
        &analysis,
        &raw_source_bytes,
        &payload_utf8,
        KINETICS_ARTIFACT_PRODUCER_COMMAND,
        env!("CARGO_PKG_VERSION"),
    )
    .map_err(|error| {
        CliError::Internal(format!("kinetics artifact preparation failed: {error}"))
    })?;
    let envelope_bytes =
        serialize_kinetics_artifact_envelope_with_limit(&envelope, MAX_KINETICS_ARTIFACT_BYTES)?;

    validate_kinetics_artifact_boundary(&envelope_bytes, &envelope, &raw_source_bytes, &analysis)?;
    write_kinetics_artifact_output_file(&args.output_path, &envelope_bytes)?;
    Ok("kinetics artifact complete\n".to_string())
}

fn validate_kinetics_artifact_output_path(output_path: &str) -> Result<(), CliError> {
    let path = Path::new(output_path);
    if path.as_os_str().is_empty()
        || output_path.ends_with('/')
        || output_path.ends_with('\\')
        || path.file_name().is_none()
    {
        return Err(CliError::User(
            "kinetics artifact output must include a file name".to_string(),
        ));
    }

    let extension = path.extension().ok_or_else(|| {
        CliError::User("kinetics artifact output must have a .json extension".to_string())
    })?;
    let extension = extension.to_str().ok_or_else(|| {
        CliError::User(
            "kinetics artifact output must have a valid UTF-8 .json extension".to_string(),
        )
    })?;
    if !extension.eq_ignore_ascii_case("json") {
        return Err(CliError::User(
            "kinetics artifact output must have a .json extension".to_string(),
        ));
    }
    Ok(())
}

fn serialize_kinetics_artifact_envelope_with_limit(
    envelope: &UnregisteredArtifactEnvelope,
    maximum: usize,
) -> Result<Vec<u8>, CliError> {
    envelope
        .to_pretty_json_bytes_with_limit(maximum)
        .map_err(|error| match error {
            ArtifactError::SerializedEnvelopeTooLarge { .. } => CliError::User(
                "kinetics artifact envelope exceeds the fixed 4 MiB output limit".to_string(),
            ),
            _ => CliError::Internal(format!(
                "kinetics artifact envelope serialization failed: {error}"
            )),
        })
}

fn validate_kinetics_artifact_boundary(
    bytes: &[u8],
    envelope: &UnregisteredArtifactEnvelope,
    raw_source_bytes: &[u8],
    analysis: &KineticsAnalysisResult,
) -> Result<(), CliError> {
    let invalid = || {
        CliError::Internal(
            "kinetics artifact envelope violated the CLI publication contract".to_string(),
        )
    };
    if bytes.len() > MAX_KINETICS_ARTIFACT_BYTES
        || bytes.starts_with(&[0xef, 0xbb, 0xbf])
        || bytes.contains(&b'\r')
    {
        return Err(invalid());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    if !text.ends_with("}\n")
        || text.ends_with("\n\n")
        || text.lines().any(|line| line.ends_with(' '))
    {
        return Err(invalid());
    }

    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let top = value.as_object().ok_or_else(&invalid)?;
    if top.len() != 3
        || !top.contains_key("schema_version")
        || !top.contains_key("artifact")
        || !top.contains_key("payload_utf8")
        || value["schema_version"] != KINETICS_ARTIFACT_ENVELOPE_SCHEMA_VERSION
    {
        return Err(invalid());
    }

    let payload_utf8 = value["payload_utf8"].as_str().ok_or_else(&invalid)?;
    let artifact = value["artifact"].as_object().ok_or_else(&invalid)?;
    let content = artifact
        .get("content")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(&invalid)?;
    let inputs = artifact
        .get("inputs")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(&invalid)?;
    let input = inputs
        .first()
        .and_then(serde_json::Value::as_object)
        .ok_or_else(&invalid)?;
    let content_hash = content
        .get("hash")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(&invalid)?;
    let input_hash = input
        .get("hash")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(&invalid)?;
    let provenance = artifact
        .get("provenance")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(&invalid)?;
    let review = artifact
        .get("review")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(&invalid)?;
    let payload_length = u64::try_from(payload_utf8.len()).map_err(|_| invalid())?;
    let input_length = u64::try_from(raw_source_bytes.len()).map_err(|_| invalid())?;
    let finding_count = u64::try_from(analysis.review.findings.len()).map_err(|_| invalid())?;
    let expected_review_status = review_status_label(analysis.review_status());

    let valid = artifact.len() == 6
        && artifact.get("kind").and_then(serde_json::Value::as_str) == Some("json")
        && artifact.get("title").and_then(serde_json::Value::as_str)
            == Some("Chemistry kinetics analysis result")
        && content.len() == 5
        && content
            .get("media_type")
            .and_then(serde_json::Value::as_str)
            == Some("application/json")
        && content
            .get("schema_version")
            .and_then(serde_json::Value::as_str)
            == Some(KINETICS_ANALYSIS_PAYLOAD_SCHEMA_VERSION)
        && content.get("encoding").and_then(serde_json::Value::as_str) == Some("utf-8")
        && content
            .get("byte_length")
            .and_then(serde_json::Value::as_u64)
            == Some(payload_length)
        && content_hash.len() == 2
        && content_hash
            .get("algorithm")
            .and_then(serde_json::Value::as_str)
            == Some("blake3")
        && content_hash
            .get("value")
            .and_then(serde_json::Value::as_str)
            == Some(hash_bytes(payload_utf8.as_bytes()).as_str())
        && inputs.len() == 1
        && input.len() == 3
        && input.get("role").and_then(serde_json::Value::as_str)
            == Some(KINETICS_ARTIFACT_SOURCE_ROLE)
        && input.get("byte_length").and_then(serde_json::Value::as_u64) == Some(input_length)
        && input_hash.len() == 2
        && input_hash
            .get("algorithm")
            .and_then(serde_json::Value::as_str)
            == Some("blake3")
        && input_hash.get("value").and_then(serde_json::Value::as_str)
            == Some(hash_bytes(raw_source_bytes).as_str())
        && provenance.len() == 4
        && provenance
            .get("workflow_id")
            .and_then(serde_json::Value::as_str)
            == Some(CHEMISTRY_KINETICS_CSV_WORKFLOW_ID)
        && provenance
            .get("workflow_step")
            .and_then(serde_json::Value::as_str)
            == Some(CHEMISTRY_KINETICS_ARTIFACT_STEP)
        && provenance
            .get("producer_command")
            .and_then(serde_json::Value::as_str)
            == Some(KINETICS_ARTIFACT_PRODUCER_COMMAND)
        && provenance
            .get("producer_version")
            .and_then(serde_json::Value::as_str)
            == Some(env!("CARGO_PKG_VERSION"))
        && review.len() == 2
        && review.get("status").and_then(serde_json::Value::as_str) == Some(expected_review_status)
        && review.get("status").and_then(serde_json::Value::as_str)
            == Some(envelope.artifact().review().status().machine_label())
        && review
            .get("finding_count")
            .and_then(serde_json::Value::as_u64)
            == Some(finding_count)
        && payload_utf8 == envelope.payload_utf8();
    if !valid {
        return Err(invalid());
    }
    Ok(())
}

fn plot_kinetics_csv(args: &KineticsPlotArgs) -> Result<String, CliError> {
    if paths_are_lexically_equal(&args.input_path, &args.output_path) {
        return Err(CliError::User(
            "input and output paths must be different".to_string(),
        ));
    }
    validate_svg_output_path(&args.output_path)?;

    let bytes = read_inspection_input(&args.input_path)?;
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(CliError::User(
            "kinetics plot input must be UTF-8 without a BOM".to_string(),
        ));
    }
    let csv_text = std::str::from_utf8(&bytes).map_err(|error| {
        CliError::User(format!(
            "kinetics plot input is not valid UTF-8 at byte offset {}",
            error.valid_up_to()
        ))
    })?;
    let table = parse_simple_numeric_csv(csv_text)
        .map_err(|error| CliError::User(format!("invalid CSV: {error}")))?;
    let columns = KineticsColumns::new(&args.time_column, &args.concentration_column)
        .map_err(format_kinetics_error)?;
    let input =
        ValidatedKineticsInput::from_table(&table, &columns).map_err(format_kinetics_error)?;
    let analysis = KineticsAnalysisResult::analyze(&input).map_err(format_kinetics_error)?;
    let plot_data = KineticsPlotData::from_analysis(&input, &columns, &analysis)
        .map_err(|error| CliError::User(format!("kinetics plot data failed: {error}")))?;
    let svg = render_kinetics_svg(&plot_data)
        .map_err(|error| CliError::User(format!("kinetics SVG rendering failed: {error}")))?;

    validate_kinetics_svg_boundary(&svg)?;
    write_svg_output_file(&args.output_path, svg.as_bytes())?;
    Ok("kinetics plot complete\n".to_string())
}

fn validate_svg_output_path(output_path: &str) -> Result<(), CliError> {
    let path = Path::new(output_path);
    if path.as_os_str().is_empty()
        || output_path.ends_with('/')
        || output_path.ends_with('\\')
        || path.file_name().is_none()
    {
        return Err(CliError::User(
            "kinetics plot output must include a file name".to_string(),
        ));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            CliError::User(
                "kinetics plot output must have a valid UTF-8 .svg extension".to_string(),
            )
        })?;
    if !extension.eq_ignore_ascii_case("svg") {
        return Err(CliError::User(
            "kinetics plot output must have a .svg extension".to_string(),
        ));
    }
    Ok(())
}

fn validate_kinetics_svg_boundary(svg: &str) -> Result<(), CliError> {
    validate_kinetics_svg_boundary_with_limit(svg, MAX_KINETICS_SVG_BYTES)
}

fn validate_kinetics_svg_boundary_with_limit(svg: &str, maximum: usize) -> Result<(), CliError> {
    let valid = svg.len() <= maximum
        && !svg.as_bytes().starts_with(&[0xef, 0xbb, 0xbf])
        && !svg.contains('\r')
        && svg.lines().next() == Some(KINETICS_SVG_ROOT_LINE)
        && svg.ends_with("</svg>\n")
        && !svg.ends_with("\n\n");
    if !valid {
        return Err(CliError::Internal(
            "kinetics SVG renderer violated the CLI publication contract".to_string(),
        ));
    }
    Ok(())
}

fn write_json_output_file(output_path: &str, bytes: &[u8]) -> Result<(), CliError> {
    let plan = plan_output_file(output_path)?;
    plan.execute(bytes).map_err(|error| {
        CliError::User(format!(
            "could not write output file `{output_path}`: {error}"
        ))
    })
}

fn write_converted_output_file(output_path: &str, bytes: &[u8]) -> Result<(), CliError> {
    let plan = plan_output_file(output_path)?;
    plan.execute(bytes)
        .map_err(|error| format_conversion_publication_error(output_path, error))
}

fn write_svg_output_file(output_path: &str, bytes: &[u8]) -> Result<(), CliError> {
    let plan = plan_output_file(output_path)?;
    plan.execute(bytes)
        .map_err(|error| format_plot_publication_error(output_path, error))
}

fn write_kinetics_artifact_output_file(output_path: &str, bytes: &[u8]) -> Result<(), CliError> {
    let plan = plan_output_file(output_path)?;
    plan.execute(bytes)
        .map_err(|error| format_artifact_publication_error(output_path, error))
}

fn plan_output_file(output_path: &str) -> Result<AtomicWritePlan, CliError> {
    let path = Path::new(output_path);
    if path.as_os_str().is_empty() {
        return Err(CliError::User("output path is empty".to_string()));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(CliError::User(format!(
            "invalid output path `{output_path}`: parent directory traversal is not allowed"
        )));
    }

    let file_name = path.file_name().ok_or_else(|| {
        CliError::User(format!(
            "invalid output path `{output_path}`: target must include a file name"
        ))
    })?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let root = StorageRoot::new(parent.to_path_buf())
        .map_err(|error| CliError::User(format!("invalid output path `{output_path}`: {error}")))?;
    let request = AtomicWriteRequest::new(PathBuf::from(file_name), Vec::<u8>::new())
        .with_write_mode(WriteMode::CreateNew);
    request
        .plan(&root)
        .map_err(|error| CliError::User(format!("invalid output path `{output_path}`: {error}")))
}

fn format_conversion_publication_error(output_path: &str, error: StorageError) -> CliError {
    match error {
        StorageError::TargetAlreadyExists { .. } => {
            CliError::User(format!("output target `{output_path}` already exists"))
        }
        StorageError::ParentDirectoryMissing { .. } => CliError::User(format!(
            "output parent directory does not exist for `{output_path}`"
        )),
        _ => CliError::User(format!(
            "conversion publication failed for `{output_path}`; the requested target may exist, inspect it before retrying"
        )),
    }
}

fn format_plot_publication_error(output_path: &str, error: StorageError) -> CliError {
    match error {
        StorageError::TargetAlreadyExists { .. } => {
            CliError::User(format!("kinetics plot output target `{output_path}` already exists"))
        }
        StorageError::ParentDirectoryMissing { .. } => CliError::User(format!(
            "kinetics plot output parent directory does not exist or is not a directory for `{output_path}`"
        )),
        _ => CliError::User(format!(
            "kinetics plot publication failed for `{output_path}`; the requested target may exist, inspect it before retrying"
        )),
    }
}

fn format_artifact_publication_error(output_path: &str, error: StorageError) -> CliError {
    match error {
        StorageError::TargetAlreadyExists { .. } => CliError::User(format!(
            "kinetics artifact output target `{output_path}` already exists"
        )),
        StorageError::ParentDirectoryMissing { .. } => CliError::User(format!(
            "kinetics artifact output parent directory does not exist or is not a directory for `{output_path}`"
        )),
        _ => CliError::User(format!(
            "kinetics artifact publication failed for `{output_path}`; the requested target may exist, inspect it before retrying"
        )),
    }
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
preferred_note: Preferred by MVP r_squared heuristic; not final scientific model selection.
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

fn format_kinetics_analysis_json_output(
    input_path: &str,
    time_column: &str,
    concentration_column: &str,
    analysis: &KineticsAnalysisResult,
) -> Result<String, CliError> {
    let first_order = analysis.comparison.first_order;
    let second_order = analysis.comparison.second_order;
    let review_findings = analysis
        .review
        .findings
        .iter()
        .map(review_finding_json)
        .collect::<Vec<_>>();
    let value = serde_json::json!({
        "schema_version": "kinetics.analysis.v1",
        "command": "kinetics.analyze",
        "input": {
            "path": input_path,
        },
        "columns": {
            "time": time_column,
            "concentration": concentration_column,
        },
        "counts": {
            "valid_points": analysis.valid_point_count(),
            "rejected_rows": analysis.rejected_row_count(),
        },
        "fits": {
            "first_order": fit_json(first_order)?,
            "second_order": fit_json(second_order)?,
        },
        "comparison": {
            "basis": comparison_basis_label(analysis.comparison_basis()),
            "preferred_model": model_kind_label(analysis.preferred_model()),
            "caution": "preferred_by_mvp_r_squared_heuristic_not_final_scientific_model_selection",
        },
        "review": {
            "status": review_status_label(analysis.review_status()),
            "findings": review_findings,
        },
    });
    let mut output = serde_json::to_string(&value)
        .map_err(|error| CliError::Internal(format!("could not format JSON output: {error}")))?;
    output.push('\n');

    Ok(output)
}

fn fit_json(
    fit: deepseek_science_chemistry::KineticsFitResult,
) -> Result<serde_json::Value, CliError> {
    Ok(serde_json::json!({
        "k": finite_json_float(fit.rate_constant_k, "fit.k")?,
        "slope": finite_json_float(fit.slope, "fit.slope")?,
        "intercept": finite_json_float(fit.intercept, "fit.intercept")?,
        "r_squared": finite_json_float(fit.r_squared, "fit.r_squared")?,
        "valid_point_count": fit.valid_point_count,
    }))
}

fn review_finding_json(
    finding: &deepseek_science_chemistry::KineticsReviewFinding,
) -> serde_json::Value {
    serde_json::json!({
        "severity": review_severity_label(finding.severity),
        "check": review_check_kind_label(finding.check_kind),
        "model": finding.model_kind.map(model_kind_label),
        "rejected_row_count": finding.rejected_row_count,
        "message": finding.message,
    })
}

fn finite_json_float(value: f64, field: &'static str) -> Result<f64, CliError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CliError::Internal(format!(
            "non-finite JSON output value: {field}"
        )))
    }
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
    use std::io::Cursor;
    use std::path::PathBuf;

    use deepseek_science_artifacts::{ArtifactError, UnregisteredArtifactEnvelope};
    use deepseek_science_chemistry::{
        prepare_kinetics_artifact_envelope, KineticsAnalysisResult, KineticsColumns,
        ValidatedKineticsInput,
    };
    use deepseek_science_common::{
        assess_simple_csv_compatibility, inspect_delimited_text, inspect_text_encoding,
        parse_simple_numeric_csv, DataColumn, DataTable, MAX_INSPECTION_BYTES,
    };
    use deepseek_science_storage::StorageError;

    use super::{
        decode_kinetics_artifact_input, format_artifact_publication_error,
        format_conversion_publication_error, format_data_conversion_report,
        format_data_inspection_report, format_kinetics_analysis_json_output,
        format_kinetics_analysis_output, format_plot_publication_error, kinetics_artifact_usage,
        kinetics_usage, parse_data_convert_args, parse_data_inspect_args,
        parse_kinetics_analyze_args, parse_kinetics_artifact_args, parse_kinetics_plot_args,
        paths_are_lexically_equal, read_bounded, read_kinetics_artifact_bounded, run_cli,
        serialize_kinetics_artifact_envelope_with_limit, validate_kinetics_artifact_boundary,
        validate_kinetics_artifact_input_metadata, validate_kinetics_artifact_output_path,
        validate_kinetics_svg_boundary_with_limit, BoundedReadError, CliError, DataConvertArgs,
        DataInspectArgs, KineticsAnalyzeArgs, KineticsArtifactArgs, KineticsPlotArgs,
        KINETICS_ARTIFACT_PRODUCER_COMMAND, KINETICS_SVG_ROOT_LINE, MAX_KINETICS_ARTIFACT_BYTES,
        MAX_KINETICS_ARTIFACT_INPUT_BYTES,
    };

    fn parse_args(args: &[&str]) -> Result<KineticsAnalyzeArgs, CliError> {
        parse_kinetics_analyze_args(args.iter().map(|value| (*value).to_string()))
    }

    fn parse_plot_args(args: &[&str]) -> Result<KineticsPlotArgs, CliError> {
        parse_kinetics_plot_args(args.iter().map(|value| (*value).to_string()))
    }

    fn parse_data_args(args: &[&str]) -> Result<DataInspectArgs, CliError> {
        parse_data_inspect_args(args.iter().map(|value| (*value).to_string()))
    }

    fn parse_convert_args(args: &[&str]) -> Result<DataConvertArgs, CliError> {
        parse_data_convert_args(args.iter().map(|value| (*value).to_string()))
    }

    fn data_report(bytes: &[u8]) -> String {
        let encoding = inspect_text_encoding(bytes).expect("test bytes should decode");
        let table = inspect_delimited_text(&encoding.text).expect("test text should inspect");
        let compatibility = assess_simple_csv_compatibility(&encoding, &table);

        format_data_inspection_report(&encoding, &table, compatibility)
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

    fn parse_artifact_args(args: &[&str]) -> Result<KineticsArtifactArgs, CliError> {
        parse_kinetics_artifact_args(args.iter().map(|value| (*value).to_string()))
    }

    fn artifact_contract() -> (
        Vec<u8>,
        KineticsAnalysisResult,
        UnregisteredArtifactEnvelope,
        Vec<u8>,
    ) {
        let raw_source = b"time_s,concentration_mol_l\n0,1\n1,0.8\n2,0.6\n".to_vec();
        let csv_text = std::str::from_utf8(&raw_source).expect("test source should be UTF-8");
        let table = parse_simple_numeric_csv(csv_text).expect("test source should parse");
        let columns = KineticsColumns::new("time_s", "concentration_mol_l")
            .expect("test columns should construct");
        let input = ValidatedKineticsInput::from_table(&table, &columns)
            .expect("test input should validate");
        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");
        let payload = format_kinetics_analysis_json_output(
            "input.csv",
            "time_s",
            "concentration_mol_l",
            &analysis,
        )
        .expect("payload should serialize");
        let envelope = prepare_kinetics_artifact_envelope(
            &analysis,
            &raw_source,
            &payload,
            KINETICS_ARTIFACT_PRODUCER_COMMAND,
            env!("CARGO_PKG_VERSION"),
        )
        .expect("envelope should construct");
        let bytes = envelope
            .to_pretty_json_bytes_with_limit(MAX_KINETICS_ARTIFACT_BYTES)
            .expect("envelope should serialize");

        (raw_source, analysis, envelope, bytes)
    }

    fn replace_once(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
        let text = std::str::from_utf8(bytes).expect("test bytes should be UTF-8");
        assert!(text.contains(from), "test replacement source should exist");
        text.replacen(from, to, 1).into_bytes()
    }

    #[test]
    fn version_command_prints_package_version() {
        let output = run_cli(["deepseek-science", "version"]);

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn data_inspect_arg_parser_accepts_input() {
        assert_eq!(
            parse_data_args(&["--input", "sample.csv"]),
            Ok(DataInspectArgs {
                input_path: "sample.csv".to_string(),
            })
        );
    }

    #[test]
    fn data_inspect_arg_parser_rejects_missing_input() {
        assert_eq!(
            parse_data_args(&[]),
            Err(CliError::User(
                "missing required argument --input".to_string()
            ))
        );
    }

    #[test]
    fn data_inspect_arg_parser_rejects_missing_input_value() {
        assert_eq!(
            parse_data_args(&["--input"]),
            Err(CliError::User("missing value for --input".to_string()))
        );
    }

    #[test]
    fn data_inspect_arg_parser_rejects_duplicate_input() {
        assert_eq!(
            parse_data_args(&["--input", "one.csv", "--input", "two.csv"]),
            Err(CliError::User("duplicate argument --input".to_string()))
        );
    }

    #[test]
    fn data_inspect_arg_parser_rejects_unknown_options() {
        assert_eq!(
            parse_data_args(&["--json"]),
            Err(CliError::User("unknown argument --json".to_string()))
        );
        assert_eq!(
            parse_data_args(&["--output", "report.txt"]),
            Err(CliError::User("unknown argument --output".to_string()))
        );
    }

    #[test]
    fn data_inspect_arg_parser_rejects_unexpected_positionals() {
        assert_eq!(
            parse_data_args(&["--input", "one.csv", "two.csv"]),
            Err(CliError::User(
                "unexpected positional argument two.csv".to_string()
            ))
        );
    }

    #[test]
    fn data_inspect_help_prints_usage_without_error() {
        let output = run_cli(["deepseek-science", "data", "inspect", "--help"]);

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("data inspect --input <path>"));
        assert!(output.stdout.contains("16 MiB"));
        assert!(output.stdout.contains("writes no files"));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn data_parent_help_prints_inspect_without_error() {
        let output = run_cli(["deepseek-science", "data", "-h"]);

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("inspect"));
        assert!(output.stdout.contains("convert"));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn data_convert_arg_parser_accepts_input_and_output() {
        assert_eq!(
            parse_convert_args(&["--input", "source.tsv", "--output", "result.csv"]),
            Ok(DataConvertArgs {
                input_path: "source.tsv".to_string(),
                output_path: "result.csv".to_string(),
            })
        );
    }

    #[test]
    fn data_convert_arg_parser_rejects_missing_and_duplicate_values() {
        assert_eq!(
            parse_convert_args(&["--output", "result.csv"]),
            Err(CliError::User(
                "missing required argument --input".to_string()
            ))
        );
        assert_eq!(
            parse_convert_args(&["--input", "source.tsv"]),
            Err(CliError::User(
                "missing required argument --output".to_string()
            ))
        );
        assert_eq!(
            parse_convert_args(&["--input"]),
            Err(CliError::User("missing value for --input".to_string()))
        );
        assert_eq!(
            parse_convert_args(&["--output"]),
            Err(CliError::User("missing value for --output".to_string()))
        );
        assert_eq!(
            parse_convert_args(&[
                "--input",
                "one.tsv",
                "--input",
                "two.tsv",
                "--output",
                "result.csv",
            ]),
            Err(CliError::User("duplicate argument --input".to_string()))
        );
        assert_eq!(
            parse_convert_args(&[
                "--input",
                "source.tsv",
                "--output",
                "one.csv",
                "--output",
                "two.csv",
            ]),
            Err(CliError::User("duplicate argument --output".to_string()))
        );
    }

    #[test]
    fn data_convert_arg_parser_rejects_unknown_options() {
        for option in ["--json", "--force", "--overwrite", "--in-place"] {
            assert_eq!(
                parse_convert_args(&[option]),
                Err(CliError::User(format!("unknown argument {option}")))
            );
        }
    }

    #[test]
    fn data_convert_help_documents_narrow_contract() {
        let output = run_cli(["deepseek-science", "data", "convert", "--help"]);

        assert_eq!(output.exit_code, 0);
        assert!(output
            .stdout
            .contains("data convert --input <path> --output <path>"));
        assert!(output.stdout.contains("16 MiB"));
        assert!(output.stdout.contains("24 MiB"));
        assert!(output.stdout.contains("exactly one final LF"));
        assert!(output.stdout.contains("never overwritten"));
        assert!(output
            .stdout
            .contains("already-compatible input is rejected"));
        assert!(output.stdout.contains("no JSON mode exists"));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn lexical_path_equality_is_checked_without_filesystem_io() {
        assert!(paths_are_lexically_equal("input.csv", "input.csv"));
        assert!(!paths_are_lexically_equal("input.csv", "output.csv"));
    }

    #[test]
    fn data_conversion_report_is_deterministic_and_has_one_newline() {
        let encoding = inspect_text_encoding(b"A\tB\n1\t2\n").expect("text should decode");
        let table = inspect_delimited_text(&encoding.text).expect("text should inspect");
        let first =
            format_data_conversion_report(&encoding, &table, 8).expect("report should format");
        let second =
            format_data_conversion_report(&encoding, &table, 8).expect("report should format");

        assert_eq!(first, second);
        assert!(first.starts_with("conversion_status: complete\n"));
        assert!(first.contains("source_delimiter: tab\n"));
        assert!(first.ends_with('\n'));
        assert!(!first.ends_with("\n\n"));
    }

    #[test]
    fn conversion_publication_errors_hide_temporary_paths() {
        let error = format_conversion_publication_error(
            "result.csv",
            StorageError::WriteFailed {
                path: PathBuf::from("secret.atomic-write.tmp"),
                reason: "denied".to_string(),
            },
        );

        assert_eq!(
            error,
            CliError::User(
                "conversion publication failed for `result.csv`; the requested target may exist, inspect it before retrying"
                    .to_string()
            )
        );
        assert!(!format!("{error:?}").contains("secret.atomic-write.tmp"));
    }

    #[test]
    fn plot_arg_parser_accepts_only_the_four_required_values() {
        assert_eq!(
            parse_plot_args(&[
                "--input",
                "input.csv",
                "--time-column",
                "time",
                "--concentration-column",
                "concentration",
                "--output",
                "output.svg",
            ]),
            Ok(KineticsPlotArgs {
                input_path: "input.csv".to_string(),
                time_column: "time".to_string(),
                concentration_column: "concentration".to_string(),
                output_path: "output.svg".to_string(),
            })
        );
        for unsupported in ["--json", "--force", "--overwrite", "--format"] {
            assert_eq!(
                parse_plot_args(&[unsupported]),
                Err(CliError::User(format!("unknown argument {unsupported}")))
            );
        }
    }

    #[test]
    fn plot_svg_boundary_validation_rejects_each_invalid_byte_contract() {
        let valid = format!("{KINETICS_SVG_ROOT_LINE}\n</svg>\n");
        assert_eq!(
            validate_kinetics_svg_boundary_with_limit(&valid, valid.len()),
            Ok(())
        );

        for invalid in [
            format!("\u{feff}{valid}"),
            valid.replace('\n', "\r\n"),
            valid.trim_end_matches('\n').to_string(),
            format!("{valid}\n"),
            "<svg>\n</svg>\n".to_string(),
        ] {
            assert!(matches!(
                validate_kinetics_svg_boundary_with_limit(&invalid, invalid.len()),
                Err(CliError::Internal(_))
            ));
        }
        assert!(matches!(
            validate_kinetics_svg_boundary_with_limit(&valid, valid.len() - 1),
            Err(CliError::Internal(_))
        ));
    }

    #[test]
    fn plot_publication_errors_are_stable_and_hide_storage_paths() {
        assert_eq!(
            format_plot_publication_error(
                "result.svg",
                StorageError::TargetAlreadyExists {
                    path: PathBuf::from("internal/result.svg"),
                },
            ),
            CliError::User("kinetics plot output target `result.svg` already exists".to_string())
        );
        assert_eq!(
            format_plot_publication_error(
                "missing/result.svg",
                StorageError::ParentDirectoryMissing {
                    path: PathBuf::from("internal/missing/result.svg"),
                },
            ),
            CliError::User(
                "kinetics plot output parent directory does not exist or is not a directory for `missing/result.svg`"
                    .to_string()
            )
        );
        let uncertain = format_plot_publication_error(
            "result.svg",
            StorageError::WriteFailed {
                path: PathBuf::from("secret.atomic-write.tmp"),
                reason: "denied".to_string(),
            },
        );
        assert_eq!(
            uncertain,
            CliError::User(
                "kinetics plot publication failed for `result.svg`; the requested target may exist, inspect it before retrying"
                    .to_string()
            )
        );
        assert!(!format!("{uncertain:?}").contains("secret.atomic-write.tmp"));
    }

    #[test]
    fn bounded_reader_accepts_exact_limit() {
        assert_eq!(
            read_bounded(Cursor::new(b"abcd"), 4).expect("exact limit should read"),
            b"abcd"
        );
    }

    #[test]
    fn bounded_reader_rejects_limit_plus_one() {
        assert!(matches!(
            read_bounded(Cursor::new(b"abcde"), 4),
            Err(BoundedReadError::LimitExceeded)
        ));
    }

    #[test]
    fn data_inspection_report_is_deterministic() {
        let bytes = b"axis,value\n1,2\n3,4\n";

        assert_eq!(data_report(bytes), data_report(bytes));
    }

    #[test]
    fn data_inspection_report_escapes_header_control_characters() {
        let output = data_report(b"safe\x1b[2J,value\n1,2\n");

        assert!(output.contains("safe\\u{1b}[2J"));
        assert!(!output.contains('\x1b'));
    }

    #[test]
    fn data_inspection_report_ends_with_one_newline() {
        let output = data_report(b"axis,value\n1,2\n");

        assert!(output.ends_with('\n'));
        assert!(!output.ends_with("\n\n"));
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
        assert_eq!(args.output_path, None);
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
    fn kinetics_analyze_arg_parser_accepts_json_flag() {
        let args = parse_args(&[
            "--input",
            "kinetics.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--json",
        ])
        .expect("valid args with --json should parse");

        assert_eq!(args.input_path, "kinetics.csv");
        assert_eq!(args.time_column, "time_s");
        assert_eq!(args.concentration_column, "concentration_mol_l");
        assert!(args.json_output);
        assert_eq!(args.output_path, None);
    }

    #[test]
    fn kinetics_analyze_arg_parser_accepts_output_path() {
        let args = parse_args(&[
            "--input",
            "kinetics.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--output",
            "result.json",
        ])
        .expect("valid args with --output should parse");

        assert_eq!(args.output_path.as_deref(), Some("result.json"));
    }

    #[test]
    fn kinetics_analyze_arg_parser_rejects_duplicate_output_path() {
        let result = parse_args(&[
            "--input",
            "kinetics.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--output",
            "result.json",
            "--output",
            "again.json",
        ]);

        assert_eq!(
            result,
            Err(CliError::User("duplicate argument --output".to_string()))
        );
    }

    #[test]
    fn kinetics_analyze_arg_parser_rejects_missing_output_value() {
        let result = parse_args(&[
            "--input",
            "kinetics.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--output",
        ]);

        assert_eq!(
            result,
            Err(CliError::User("missing value for --output".to_string()))
        );
    }

    #[test]
    fn kinetics_analyze_help_prints_usage_without_error() {
        let output = run_cli(["deepseek-science", "kinetics", "analyze", "--help"]);

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("Usage:"));
        assert!(output.stdout.contains("--input <path>"));
        assert!(output.stdout.contains("--time-column <column>"));
        assert!(output.stdout.contains("--concentration-column <column>"));
        assert!(output.stdout.contains("--json"));
        assert!(output.stdout.contains("--output <path>"));
        assert!(output.stdout.contains("Text output is the default."));
        assert!(output
            .stdout
            .contains("Existing targets are not overwritten"));
        assert!(output.stdout.contains("parent directories are not created"));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn kinetics_analyze_short_help_prints_usage_without_error() {
        let output = run_cli(["deepseek-science", "kinetics", "analyze", "-h"]);

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("Usage:"));
        assert!(output.stdout.contains("--json"));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn kinetics_analyze_arg_parser_rejects_duplicate_json_flag() {
        let result = parse_args(&[
            "--input",
            "kinetics.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--json",
            "--json",
        ]);

        assert_eq!(
            result,
            Err(CliError::User("duplicate argument --json".to_string()))
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
            "--wat",
        ]);

        assert_eq!(
            result,
            Err(CliError::User("unknown argument --wat".to_string()))
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
    fn kinetics_analyze_unknown_argument_reports_useful_error() {
        let output = run_cli([
            "deepseek-science",
            "kinetics",
            "analyze",
            "--input",
            "kinetics.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--wat",
        ]);

        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("unknown argument --wat"));
        assert!(output.stderr.contains("Usage:"));
        assert_eq!(output.stdout, "");
    }

    #[test]
    fn kinetics_analyze_missing_required_argument_reports_useful_error() {
        let output = run_cli([
            "deepseek-science",
            "kinetics",
            "analyze",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ]);

        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("missing required argument --input"));
        assert!(output.stderr.contains("Usage:"));
        assert_eq!(output.stdout, "");
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
        assert!(!output.contains("definitive"));
        assert!(!output.contains("true model"));
        assert!(!output.contains("proved first-order"));
    }

    #[test]
    fn artifact_routing_and_help_are_frozen() {
        assert!(super::usage().contains("kinetics artifact"));
        assert!(kinetics_usage().contains("<analyze|plot|artifact>"));
        assert!(kinetics_usage().contains("artifact"));

        for flag in ["--help", "-h"] {
            let output = run_cli(["deepseek-science", "kinetics", "artifact", flag]);
            assert_eq!(output.exit_code, 0);
            assert_eq!(output.stderr, "");
            assert_eq!(output.stdout, kinetics_artifact_usage());
        }
        let mixed = run_cli([
            "deepseek-science",
            "kinetics",
            "artifact",
            "--help",
            "--input",
            "input.csv",
        ]);
        assert_ne!(mixed.exit_code, 0);
        assert_eq!(mixed.stdout, "");
        assert!(mixed.stderr.contains("unknown argument --help"));
    }

    #[test]
    fn artifact_arg_parser_accepts_exactly_four_required_options() {
        assert_eq!(
            parse_artifact_args(&[
                "--input",
                "input.csv",
                "--time-column",
                "time",
                "--concentration-column",
                "concentration",
                "--output",
                "result.json",
            ]),
            Ok(KineticsArtifactArgs {
                input_path: "input.csv".to_string(),
                time_column: "time".to_string(),
                concentration_column: "concentration".to_string(),
                output_path: "result.json".to_string(),
            })
        );
    }

    #[test]
    fn artifact_arg_parser_rejects_each_duplicate_and_missing_option() {
        let valid = [
            "--input",
            "input.csv",
            "--time-column",
            "time",
            "--concentration-column",
            "concentration",
            "--output",
            "result.json",
        ];
        for option in [
            "--input",
            "--time-column",
            "--concentration-column",
            "--output",
        ] {
            let index = valid.iter().position(|value| value == &option).unwrap();
            let mut duplicate = valid.to_vec();
            duplicate.extend([option, valid[index + 1]]);
            assert_eq!(
                parse_artifact_args(&duplicate),
                Err(CliError::User(format!("duplicate argument {option}")))
            );

            let missing = valid
                .iter()
                .enumerate()
                .filter(|(position, _)| *position != index && *position != index + 1)
                .map(|(_, value)| *value)
                .collect::<Vec<_>>();
            assert_eq!(
                parse_artifact_args(&missing),
                Err(CliError::User(format!(
                    "missing required argument {option}"
                )))
            );
        }
    }

    #[test]
    fn artifact_arg_parser_rejects_missing_values_unknowns_and_positionals() {
        for option in [
            "--input",
            "--time-column",
            "--concentration-column",
            "--output",
        ] {
            assert_eq!(
                parse_artifact_args(&[option]),
                Err(CliError::User(format!("missing value for {option}")))
            );
            assert_eq!(
                parse_artifact_args(&[option, ""]),
                Err(CliError::User(format!("missing value for {option}")))
            );
        }
        for option in ["--json", "--overwrite", "--force", "--rag"] {
            assert_eq!(
                parse_artifact_args(&[option]),
                Err(CliError::User(format!("unknown argument {option}")))
            );
        }
        assert_eq!(
            parse_artifact_args(&["input.csv"]),
            Err(CliError::User(
                "unexpected positional argument input.csv".to_string()
            ))
        );
    }

    #[test]
    fn artifact_output_path_validation_is_lexical_and_json_specific() {
        for output in ["result.json", "result.JSON", "result.JsOn"] {
            assert_eq!(validate_kinetics_artifact_output_path(output), Ok(()));
        }
        for output in ["result", "result.svg", "result.json.txt"] {
            assert_eq!(
                validate_kinetics_artifact_output_path(output),
                Err(CliError::User(
                    "kinetics artifact output must have a .json extension".to_string()
                ))
            );
        }
        for output in ["", "directory/", "directory\\"] {
            assert_eq!(
                validate_kinetics_artifact_output_path(output),
                Err(CliError::User(
                    "kinetics artifact output must include a file name".to_string()
                ))
            );
        }
        assert!(paths_are_lexically_equal("same.json", "same.json"));
    }

    #[test]
    fn artifact_input_and_output_limits_are_frozen() {
        assert_eq!(MAX_KINETICS_ARTIFACT_INPUT_BYTES, MAX_INSPECTION_BYTES);
        assert_eq!(MAX_KINETICS_ARTIFACT_INPUT_BYTES, 16 * 1024 * 1024);
        assert_eq!(MAX_KINETICS_ARTIFACT_BYTES, 4 * 1024 * 1024);
    }

    #[test]
    fn artifact_bounded_reader_and_metadata_checks_use_inclusive_limits() {
        assert_eq!(
            read_kinetics_artifact_bounded(Cursor::new(b"abcd"), "input.csv", 4)
                .expect("exact limit should succeed"),
            b"abcd"
        );
        assert_eq!(
            read_kinetics_artifact_bounded(Cursor::new(b"abcde"), "input.csv", 4),
            Err(CliError::User(
                "kinetics artifact input file `input.csv` exceeds the fixed 16 MiB limit"
                    .to_string()
            ))
        );
        assert_eq!(
            validate_kinetics_artifact_input_metadata("input.csv", true, 4, 4),
            Ok(())
        );
        assert_eq!(
            validate_kinetics_artifact_input_metadata("input.csv", true, 5, 4),
            Err(CliError::User(
                "kinetics artifact input file `input.csv` exceeds the fixed 16 MiB limit"
                    .to_string()
            ))
        );
        assert_eq!(
            validate_kinetics_artifact_input_metadata("input.csv", false, 0, 4),
            Err(CliError::User(
                "kinetics artifact input path `input.csv` must refer to a regular file".to_string()
            ))
        );
    }

    #[test]
    fn artifact_utf8_decoder_rejects_bom_and_reports_invalid_offset() {
        assert_eq!(decode_kinetics_artifact_input(b"abc\n"), Ok("abc\n"));
        assert_eq!(
            decode_kinetics_artifact_input(b"\xef\xbb\xbfabc\n"),
            Err(CliError::User(
                "kinetics artifact input must be UTF-8 without a BOM".to_string()
            ))
        );
        assert_eq!(
            decode_kinetics_artifact_input(b"a\xff"),
            Err(CliError::User(
                "kinetics artifact input is not valid UTF-8 at byte offset 1".to_string()
            ))
        );
    }

    #[test]
    fn artifact_envelope_serialization_has_an_exact_limit_seam() {
        let (raw_source, analysis, envelope, expected) = artifact_contract();
        let exact = serialize_kinetics_artifact_envelope_with_limit(&envelope, expected.len())
            .expect("exact limit should succeed");
        assert_eq!(exact, expected);
        assert_eq!(
            serialize_kinetics_artifact_envelope_with_limit(&envelope, expected.len() - 1),
            Err(CliError::User(
                "kinetics artifact envelope exceeds the fixed 4 MiB output limit".to_string()
            ))
        );
        assert_eq!(
            validate_kinetics_artifact_boundary(&exact, &envelope, &raw_source, &analysis),
            Ok(())
        );
    }

    #[test]
    fn artifact_postcondition_rejects_invalid_outer_byte_contracts() {
        let (raw_source, analysis, envelope, bytes) = artifact_contract();
        let mut bom = b"\xef\xbb\xbf".to_vec();
        bom.extend_from_slice(&bytes);
        let cr = replace_once(&bytes, "\n", "\r\n");
        let missing_lf = bytes[..bytes.len() - 1].to_vec();
        let mut double_lf = bytes.clone();
        double_lf.push(b'\n');

        for invalid in [bom, cr, missing_lf, double_lf] {
            assert!(matches!(
                validate_kinetics_artifact_boundary(&invalid, &envelope, &raw_source, &analysis,),
                Err(CliError::Internal(_))
            ));
        }
    }

    #[test]
    fn artifact_postcondition_rejects_wrong_schema_hashes_and_producer_version() {
        let (raw_source, analysis, envelope, bytes) = artifact_contract();
        let payload_hash = envelope.artifact().content().hash().value();
        let input_hash = envelope.artifact().inputs()[0].hash().value();
        let invalid = [
            replace_once(&bytes, "kinetics.artifact.v1", "kinetics.artifact.v0"),
            replace_once(&bytes, payload_hash, &"0".repeat(64)),
            replace_once(&bytes, input_hash, &"f".repeat(64)),
            replace_once(&bytes, env!("CARGO_PKG_VERSION"), "wrong-version"),
        ];

        for invalid in invalid {
            assert!(matches!(
                validate_kinetics_artifact_boundary(&invalid, &envelope, &raw_source, &analysis,),
                Err(CliError::Internal(_))
            ));
        }
    }

    #[test]
    fn artifact_publication_errors_are_stable_and_hide_storage_paths() {
        assert_eq!(
            format_artifact_publication_error(
                "result.json",
                StorageError::TargetAlreadyExists {
                    path: PathBuf::from("internal/result.json"),
                },
            ),
            CliError::User(
                "kinetics artifact output target `result.json` already exists".to_string()
            )
        );
        assert_eq!(
            format_artifact_publication_error(
                "missing/result.json",
                StorageError::ParentDirectoryMissing {
                    path: PathBuf::from("internal/missing/result.json"),
                },
            ),
            CliError::User(
                "kinetics artifact output parent directory does not exist or is not a directory for `missing/result.json`"
                    .to_string()
            )
        );
        let uncertain = format_artifact_publication_error(
            "result.json",
            StorageError::WriteFailed {
                path: PathBuf::from("secret.atomic-write.tmp"),
                reason: "denied".to_string(),
            },
        );
        assert_eq!(
            uncertain,
            CliError::User(
                "kinetics artifact publication failed for `result.json`; the requested target may exist, inspect it before retrying"
                    .to_string()
            )
        );
        assert!(!format!("{uncertain:?}").contains("secret.atomic-write.tmp"));
    }

    #[test]
    fn artifact_serializer_maps_generic_size_overflow_without_partial_bytes() {
        let (_, _, envelope, _) = artifact_contract();
        assert_eq!(
            envelope.to_pretty_json_bytes_with_limit(0),
            Err(ArtifactError::SerializedEnvelopeTooLarge { maximum: 0 })
        );
        assert_eq!(
            serialize_kinetics_artifact_envelope_with_limit(&envelope, 0),
            Err(CliError::User(
                "kinetics artifact envelope exceeds the fixed 4 MiB output limit".to_string()
            ))
        );
    }
}
