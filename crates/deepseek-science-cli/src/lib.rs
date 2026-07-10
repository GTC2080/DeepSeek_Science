#![forbid(unsafe_code)]
//! Minimal command handling for the `deepseek-science` binary.
//!
//! The CLI intentionally uses `std::env::args` in Phase 1 to avoid pulling in a
//! command-line framework before the command surface exists.

use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use deepseek_science_artifacts::hash_bytes;
use deepseek_science_chemistry::{
    KineticsAnalysisResult, KineticsColumns, KineticsComparisonBasis, KineticsError,
    KineticsModelKind, KineticsReviewCheckKind, KineticsReviewSeverity, KineticsReviewStatus,
    ValidatedKineticsInput,
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
    "Usage: deepseek-science kinetics <analyze>\n"
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

    use deepseek_science_chemistry::{
        KineticsAnalysisResult, KineticsColumns, ValidatedKineticsInput,
    };
    use deepseek_science_common::{
        assess_simple_csv_compatibility, inspect_delimited_text, inspect_text_encoding, DataColumn,
        DataTable,
    };
    use deepseek_science_storage::StorageError;

    use super::{
        format_conversion_publication_error, format_data_conversion_report,
        format_data_inspection_report, format_kinetics_analysis_output, parse_data_convert_args,
        parse_data_inspect_args, parse_kinetics_analyze_args, paths_are_lexically_equal,
        read_bounded, run_cli, BoundedReadError, CliError, DataConvertArgs, DataInspectArgs,
        KineticsAnalyzeArgs,
    };

    fn parse_args(args: &[&str]) -> Result<KineticsAnalyzeArgs, CliError> {
        parse_kinetics_analyze_args(args.iter().map(|value| (*value).to_string()))
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
}
