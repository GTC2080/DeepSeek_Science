use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn create_test_directory(label: &str) -> PathBuf {
    let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let target_tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .canonicalize()
        .expect("Cargo target temp directory should be resolvable");
    let directory = target_tmp.join(format!(
        "deepseek-science-data-inspect-{label}-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("test directory should be unique");

    directory
}

fn create_input(directory: &Path, bytes: &[u8]) -> PathBuf {
    let input = directory.join("input.txt");
    fs::write(&input, bytes).expect("test input should be written");
    input
}

fn directory_paths(directory: &Path) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(directory)
        .expect("test directory should be readable")
        .map(|entry| entry.expect("test entry should be readable").path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn cleanup_input(directory: &Path, input: &Path) {
    assert_eq!(directory_paths(directory), vec![input.to_path_buf()]);
    fs::remove_file(input).expect("exact input cleanup should succeed");
    fs::remove_dir(directory).expect("exact empty directory cleanup should succeed");
}

fn cleanup_empty_directory(directory: &Path) {
    assert!(directory_paths(directory).is_empty());
    fs::remove_dir(directory).expect("exact empty directory cleanup should succeed");
}

fn run_data_inspect(input: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["data", "inspect", "--input"])
        .arg(input)
        .output()
        .expect("CLI process should run")
}

fn run_data_inspect_in(directory: &Path, input: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .current_dir(directory)
        .args(["data", "inspect", "--input", input])
        .output()
        .expect("CLI process should run")
}

fn run_data_inspect_help(directory: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .current_dir(directory)
        .args(["data", "inspect", "--help"])
        .output()
        .expect("CLI process should run")
}

fn output_text(output: std::process::Output) -> (std::process::ExitStatus, String, String) {
    let status = output.status;
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");

    (status, stdout, stderr)
}

fn assert_success(status: std::process::ExitStatus, stdout: &str, stderr: &str) {
    assert!(status.success(), "expected success, stderr: {stderr}");
    assert!(stdout.starts_with("inspection_status: complete\n"));
    assert!(stdout.ends_with('\n'));
    assert!(!stdout.ends_with("\n\n"));
    assert_eq!(stderr, "");
}

fn utf16_bytes(text: &str, little_endian: bool) -> Vec<u8> {
    let mut bytes = if little_endian {
        vec![0xFF, 0xFE]
    } else {
        vec![0xFE, 0xFF]
    };
    for unit in text.encode_utf16() {
        let encoded = if little_endian {
            unit.to_le_bytes()
        } else {
            unit.to_be_bytes()
        };
        bytes.extend_from_slice(&encoded);
    }
    bytes
}

#[test]
fn utf8_comma_narrow_table_is_inspected_read_only() {
    let directory = create_test_directory("utf8-comma");
    let input = create_input(&directory, b"axis,value\n0,1\n1,0.8\n");
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("encoding: utf-8\n"));
    assert!(stdout.contains("bom: none\n"));
    assert!(stdout.contains("delimiter: comma\n"));
    assert!(stdout.contains("shape: numeric-narrow-table\n"));
    assert!(stdout.contains("simple_csv_compatibility: compatible-as-is\n"));
    assert!(stdout.contains(
        "current_kinetics_workflow: potentially-compatible-after-explicit-column-selection\n"
    ));
    cleanup_input(&directory, &input);
}

#[test]
fn utf8_bom_comma_table_requires_normalization() {
    let directory = create_test_directory("utf8-bom");
    let input = create_input(&directory, b"\xEF\xBB\xBFaxis,value\n0,1\n1,2\n");
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("encoding: utf-8\n"));
    assert!(stdout.contains("bom: utf-8\n"));
    assert!(stdout.contains("simple_csv_compatibility: requires-explicit-normalization\n"));
    assert!(stdout.contains("normalization is required but is not implemented"));
    cleanup_input(&directory, &input);
}

#[test]
fn utf16le_comma_table_requires_normalization() {
    let directory = create_test_directory("utf16le");
    let input = create_input(&directory, &utf16_bytes("axis,value\n0,1\n1,2\n", true));
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("encoding: utf-16le\n"));
    assert!(stdout.contains("bom: utf-16le\n"));
    assert!(stdout.contains("simple_csv_compatibility: requires-explicit-normalization\n"));
    cleanup_input(&directory, &input);
}

#[test]
fn utf16be_comma_table_requires_normalization() {
    let directory = create_test_directory("utf16be");
    let input = create_input(&directory, &utf16_bytes("axis,value\n0,1\n1,2\n", false));
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("encoding: utf-16be\n"));
    assert!(stdout.contains("bom: utf-16be\n"));
    assert!(stdout.contains("simple_csv_compatibility: requires-explicit-normalization\n"));
    cleanup_input(&directory, &input);
}

#[test]
fn utf8_tab_table_requires_normalization() {
    let directory = create_test_directory("utf8-tab");
    let input = create_input(&directory, b"axis\tvalue\n0\t1\n1\t2\n");
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("delimiter: tab\n"));
    assert!(stdout.contains("shape: numeric-narrow-table\n"));
    assert!(stdout.contains("simple_csv_compatibility: requires-explicit-normalization\n"));
    cleanup_input(&directory, &input);
}

#[test]
fn generic_numeric_matrix_is_incompatible_without_column_inference() {
    let directory = create_test_directory("matrix");
    let input = create_input(&directory, b"axis,series 1,series 2\n0,1,2\n1,3,4\n2,5,6\n");
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("shape: numeric-matrix\n"));
    assert!(stdout.contains("current_kinetics_workflow: incompatible\n"));
    assert!(!stdout.contains("time_column:"));
    assert!(!stdout.contains("concentration_column:"));
    cleanup_input(&directory, &input);
}

#[test]
fn unit_row_is_a_successful_mixed_structure_finding() {
    let directory = create_test_directory("unit-row");
    let input = create_input(
        &directory,
        b"axis,value,condition\ns,unit,C\n0,1,25\n1,2,25\n",
    );
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("shape: mixed-or-unsupported\n"));
    assert!(stdout.contains("multiple header or unit rows"));
    assert!(stdout.contains("current_kinetics_workflow: incompatible\n"));
    cleanup_input(&directory, &input);
}

#[test]
fn empty_file_is_a_successful_empty_finding() {
    let directory = create_test_directory("empty");
    let input = create_input(&directory, b"");
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("shape: empty\n"));
    assert!(stdout.contains("table_region: none\n"));
    assert!(stdout.contains("current_kinetics_workflow: incompatible\n"));
    cleanup_input(&directory, &input);
}

#[test]
fn quoted_field_is_a_successful_unsupported_finding() {
    let directory = create_test_directory("quoted");
    let input = create_input(&directory, b"axis,value\n0,\"1\"\n");
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("delimiter: unsupported\n"));
    assert!(stdout.contains("shape: mixed-or-unsupported\n"));
    assert!(stdout.contains("quoted_lines: 2\n"));
    assert!(stdout.contains("quoted or multiline field parsing is unsupported"));
    cleanup_input(&directory, &input);
}

#[test]
fn comma_tab_ambiguity_is_a_successful_incompatible_finding() {
    let directory = create_test_directory("ambiguous");
    let input = create_input(&directory, b"A,B\n1,2\n\nC\tD\n3\t4\n");
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert_success(status, &stdout, &stderr);
    assert!(stdout.contains("delimiter: ambiguous\n"));
    assert!(stdout.contains("shape: mixed-or-unsupported\n"));
    assert!(stdout.contains("current_kinetics_workflow: incompatible\n"));
    cleanup_input(&directory, &input);
}

#[test]
fn invalid_bom_free_utf8_is_a_fatal_encoding_error() {
    let directory = create_test_directory("invalid-utf8");
    let input = create_input(&directory, &[b'A', 0xFF]);
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("unsupported or ambiguous encoding"));
    assert!(stderr.contains("BOM-free UTF-16 is not detected"));
    cleanup_input(&directory, &input);
}

#[test]
fn invalid_utf16_is_a_fatal_error_with_original_byte_offset() {
    let directory = create_test_directory("invalid-utf16");
    let input = create_input(&directory, &[0xFF, 0xFE, 0x00, 0xD8]);
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("invalid UTF-16"));
    assert!(stderr.contains("byte offset 2"));
    cleanup_input(&directory, &input);
}

#[test]
fn missing_file_is_a_concise_fatal_error() {
    let directory = create_test_directory("missing");
    let input = directory.join("missing.txt");
    let (status, stdout, stderr) = output_text(run_data_inspect(&input));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("could not open input file"));
    assert!(!stderr.contains("Usage:"));
    cleanup_empty_directory(&directory);
}

#[test]
fn directory_input_requires_a_regular_file() {
    let directory = create_test_directory("directory");
    let (status, stdout, stderr) = output_text(run_data_inspect(&directory));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("must refer to a regular file"));
    cleanup_empty_directory(&directory);
}

#[test]
fn help_documents_read_only_bounded_behavior() {
    let directory = create_test_directory("help");
    let (status, stdout, stderr) = output_text(run_data_inspect_help(&directory));

    assert!(status.success());
    assert!(stdout.contains("data inspect --input <path>"));
    assert!(stdout.contains("16 MiB"));
    assert!(stdout.contains("UTF-16LE/BE with a BOM"));
    assert!(stdout.contains("Only comma and tab"));
    assert!(stdout.contains("writes no files"));
    assert!(stdout.contains("does not modify, normalize, convert, or analyze"));
    assert!(stdout.contains("reported rather than repaired"));
    assert!(!stdout.contains("data convert"));
    assert_eq!(stderr, "");
    cleanup_empty_directory(&directory);
}

#[test]
fn repeated_inspection_produces_identical_output() {
    let directory = create_test_directory("repeated");
    let input = create_input(&directory, b"axis,value\n0,1\n1,2\n");
    let first = run_data_inspect(&input);
    let second = run_data_inspect(&input);

    assert!(first.status.success());
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, b"");
    assert_eq!(second.stderr, b"");
    cleanup_input(&directory, &input);
}

#[test]
fn relative_input_creates_no_side_effects() {
    let directory = create_test_directory("no-side-effects");
    let input = create_input(&directory, b"axis,value\n0,1\n1,2\n");
    let (status, stdout, stderr) = output_text(run_data_inspect_in(&directory, "input.txt"));

    assert_success(status, &stdout, &stderr);
    assert_eq!(directory_paths(&directory), vec![input.clone()]);
    cleanup_input(&directory, &input);
}
