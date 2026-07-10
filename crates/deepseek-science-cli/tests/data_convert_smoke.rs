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
        "deepseek-science-data-convert-{label}-{}-{sequence}",
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

fn sorted_paths(paths: &[&Path]) -> Vec<PathBuf> {
    let mut paths = paths
        .iter()
        .map(|path| (*path).to_path_buf())
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn cleanup_files(directory: &Path, files: &[&Path]) {
    assert_eq!(directory_paths(directory), sorted_paths(files));
    for file in files {
        fs::remove_file(file).expect("exact file cleanup should succeed");
    }
    fs::remove_dir(directory).expect("exact empty directory cleanup should succeed");
}

fn cleanup_empty_directory(directory: &Path) {
    assert!(directory_paths(directory).is_empty());
    fs::remove_dir(directory).expect("exact empty directory cleanup should succeed");
}

fn run_convert(input: &Path, output: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["data", "convert", "--input"])
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .expect("CLI process should run")
}

fn run_args(directory: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .current_dir(directory)
        .args(args)
        .output()
        .expect("CLI process should run")
}

fn output_text(output: std::process::Output) -> (std::process::ExitStatus, String, String) {
    let status = output.status;
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    (status, stdout, stderr)
}

fn utf16_bytes(text: &str, little_endian: bool) -> Vec<u8> {
    let mut bytes = if little_endian {
        vec![0xff, 0xfe]
    } else {
        vec![0xfe, 0xff]
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

fn assert_successful_conversion(label: &str, input_bytes: &[u8], expected: &[u8]) -> String {
    let directory = create_test_directory(label);
    let input = create_input(&directory, input_bytes);
    let output = directory.join("output.csv");
    let (status, stdout, stderr) = output_text(run_convert(&input, &output));

    assert!(status.success(), "conversion failed: {stderr}");
    assert_eq!(stderr, "");
    assert!(stdout.starts_with("conversion_status: complete\n"));
    assert!(stdout.ends_with('\n'));
    assert!(!stdout.ends_with("\n\n"));
    assert_eq!(fs::read(&output).expect("output should exist"), expected);
    assert_eq!(
        fs::read(&input).expect("input should remain readable"),
        input_bytes
    );
    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &output])
    );

    cleanup_files(&directory, &[&input, &output]);
    stdout
}

fn assert_rejected(label: &str, input_bytes: &[u8], message: &str) {
    let directory = create_test_directory(label);
    let input = create_input(&directory, input_bytes);
    let output = directory.join("output.csv");
    let (status, stdout, stderr) = output_text(run_convert(&input, &output));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains(message), "unexpected stderr: {stderr}");
    assert_eq!(
        fs::read(&input).expect("input should remain readable"),
        input_bytes
    );
    assert_eq!(directory_paths(&directory), vec![input.clone()]);

    cleanup_files(&directory, &[&input]);
}

#[test]
fn utf8_bom_comma_conversion_publishes_exact_bytes() {
    let stdout = assert_successful_conversion(
        "utf8-bom-comma",
        b"\xef\xbb\xbfA,B\r\n+01.00e-03,-0\r\n",
        b"A,B\n+01.00e-03,-0\n",
    );

    assert!(stdout.contains("source_encoding: utf-8\n"));
    assert!(stdout.contains("source_bom: utf-8\n"));
    assert!(stdout.contains("source_delimiter: comma\n"));
    assert!(stdout.contains("output_encoding: utf-8\n"));
    assert!(stdout.contains("output_bom: none\n"));
    assert!(stdout.contains("line_endings: lf\n"));
}

#[test]
fn utf16le_comma_conversion_succeeds() {
    assert_successful_conversion(
        "utf16le-comma",
        &utf16_bytes("A,B\n1,2\n", true),
        b"A,B\n1,2\n",
    );
}

#[test]
fn utf16be_comma_conversion_succeeds() {
    assert_successful_conversion(
        "utf16be-comma",
        &utf16_bytes("A,B\n1,2", false),
        b"A,B\n1,2\n",
    );
}

#[test]
fn utf8_tab_conversion_preserves_unicode_and_lexical_numbers() {
    assert_successful_conversion(
        "utf8-tab",
        "波长\t值\n1E+2\t+01.00e-03\n".as_bytes(),
        "波长,值\n1E+2,+01.00e-03\n".as_bytes(),
    );
}

#[test]
fn utf16_tab_conversion_succeeds() {
    assert_successful_conversion(
        "utf16-tab",
        &utf16_bytes("A\tB\r\n1\t2\r\n", true),
        b"A,B\n1,2\n",
    );
}

#[test]
fn repeated_conversion_to_fresh_targets_is_byte_identical() {
    let directory = create_test_directory("repeated");
    let input_bytes = b"A\tB\n1\t2\n";
    let input = create_input(&directory, input_bytes);
    let first = directory.join("first.csv");
    let second = directory.join("second.csv");

    let first_result = output_text(run_convert(&input, &first));
    let second_result = output_text(run_convert(&input, &second));
    assert!(first_result.0.success());
    assert!(second_result.0.success());
    assert_eq!(first_result.2, "");
    assert_eq!(second_result.2, "");
    assert_eq!(
        fs::read(&first).expect("first output"),
        fs::read(&second).expect("second output")
    );
    assert_eq!(fs::read(&input).expect("input should remain"), input_bytes);
    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &first, &second])
    );

    cleanup_files(&directory, &[&input, &first, &second]);
}

#[test]
fn already_compatible_input_creates_no_output() {
    assert_rejected("already-compatible", b"A,B\n1,2\n", "can be used directly");
}

#[test]
fn lexically_identical_paths_are_rejected_before_output_access() {
    let directory = create_test_directory("identical");
    let input_bytes = b"A\tB\n1\t2\n";
    let input = create_input(&directory, input_bytes);
    let (status, stdout, stderr) = output_text(run_convert(&input, &input));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("input and output paths must be different"));
    assert_eq!(fs::read(&input).expect("input should remain"), input_bytes);
    cleanup_files(&directory, &[&input]);
}

#[test]
fn existing_target_retains_sentinel_bytes() {
    const SENTINEL: &[u8] = b"existing target";

    let directory = create_test_directory("existing-target");
    let input = create_input(&directory, b"A\tB\n1\t2\n");
    let output = directory.join("output.csv");
    fs::write(&output, SENTINEL).expect("sentinel should be written");
    let (status, stdout, stderr) = output_text(run_convert(&input, &output));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("already exists"));
    assert_eq!(fs::read(&output).expect("target should remain"), SENTINEL);
    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &output])
    );
    cleanup_files(&directory, &[&input, &output]);
}

#[test]
fn missing_output_parent_is_not_created() {
    let directory = create_test_directory("missing-parent");
    let input = create_input(&directory, b"A\tB\n1\t2\n");
    let missing_parent = directory.join("missing");
    let output = missing_parent.join("output.csv");
    let (status, stdout, stderr) = output_text(run_convert(&input, &output));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("output parent directory does not exist"));
    assert!(!missing_parent.exists());
    cleanup_files(&directory, &[&input]);
}

#[test]
fn directory_and_missing_inputs_are_rejected() {
    let directory = create_test_directory("input-errors");
    let output = directory.join("output.csv");

    let directory_result = output_text(run_convert(&directory, &output));
    assert!(!directory_result.0.success());
    assert_eq!(directory_result.1, "");
    assert!(directory_result.2.contains("must refer to a regular file"));

    let missing = directory.join("missing.txt");
    let missing_result = output_text(run_convert(&missing, &output));
    assert!(!missing_result.0.success());
    assert_eq!(missing_result.1, "");
    assert!(missing_result.2.contains("could not open input file"));
    cleanup_empty_directory(&directory);
}

#[test]
fn invalid_encoding_is_rejected() {
    assert_rejected(
        "invalid-encoding",
        &[b'A', 0xff],
        "unsupported or ambiguous encoding",
    );
}

#[test]
fn matrix_metadata_and_unit_rows_are_rejected() {
    assert_rejected(
        "matrix",
        b"\xef\xbb\xbfaxis,series 1,series 2\n1,2,3\n2,3,4\n",
        "not eligible",
    );
    assert_rejected(
        "metadata",
        b"\xef\xbb\xbfmetadata\nA,B\n1,2\n",
        "not eligible",
    );
    assert_rejected(
        "unit-row",
        b"\xef\xbb\xbfA,B\nunit,unit\n1,2\n",
        "not eligible",
    );
}

#[test]
fn blank_and_quoted_rows_are_rejected() {
    assert_rejected(
        "blank-row",
        b"\xef\xbb\xbfA,B\n1,2\n\n3,4\n",
        "not eligible",
    );
    assert_rejected("quoted-row", b"\xef\xbb\xbfA,B\n1,\"2\"\n", "not eligible");
}

#[test]
fn whitespace_and_control_cells_are_rejected() {
    assert_rejected(
        "whitespace-cell",
        b"A\tB\n1\t 2\n",
        "unsafe cell content at line 2, column 2",
    );
    assert_rejected(
        "control-cell",
        b"A\x01\tB\n1\t2\n",
        "ASCII control characters are not supported",
    );
}

#[test]
fn ambiguous_delimiter_is_rejected() {
    assert_rejected(
        "ambiguous",
        b"\xef\xbb\xbfA,B\n1,2\n\nC\tD\n3\t4\n",
        "not eligible",
    );
}

#[test]
fn unknown_and_duplicate_options_fail_without_side_effects() {
    let directory = create_test_directory("argument-errors");

    let unknown = output_text(run_args(&directory, &["data", "convert", "--json"]));
    assert!(!unknown.0.success());
    assert_eq!(unknown.1, "");
    assert!(unknown.2.contains("unknown argument --json"));

    let input = create_input(&directory, b"A\tB\n1\t2\n");
    let duplicate = output_text(run_args(
        &directory,
        &[
            "data",
            "convert",
            "--input",
            "input.txt",
            "--input",
            "input.txt",
            "--output",
            "output.csv",
        ],
    ));
    assert!(!duplicate.0.success());
    assert_eq!(duplicate.1, "");
    assert!(duplicate.2.contains("duplicate argument --input"));
    assert_eq!(directory_paths(&directory), vec![input.clone()]);
    cleanup_files(&directory, &[&input]);
}

#[test]
fn help_documents_conversion_without_writing_files() {
    let directory = create_test_directory("help");
    let output = output_text(run_args(&directory, &["data", "convert", "--help"]));

    assert!(output.0.success());
    assert_eq!(output.2, "");
    assert!(output.1.contains("16 MiB"));
    assert!(output.1.contains("24 MiB"));
    assert!(output.1.contains("exactly one final LF"));
    assert!(output.1.contains("never overwritten"));
    assert!(directory_paths(&directory).is_empty());
    cleanup_empty_directory(&directory);
}
