use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use deepseek_science_artifacts::hash_bytes;
use serde_json::Value;

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const VALID_CSV: &[u8] = b"time_s,concentration_mol_l\n0,1\n1,0.8\n2,0.6\n";
const WARNING_CSV: &[u8] = b"time_s,concentration_mol_l\n0,1\n1,0\n2,0.6\n";

static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn create_test_directory(label: &str) -> PathBuf {
    let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let target_tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .canonicalize()
        .expect("Cargo target temp directory should be resolvable");
    let directory = target_tmp.join(format!(
        "deepseek-science-kinetics-artifact-{label}-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("test directory should be unique");
    directory
}

fn create_input(directory: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let input = directory.join(name);
    fs::write(&input, bytes).expect("test input should be written");
    input
}

fn run_args(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .current_dir(directory)
        .args(args)
        .output()
        .expect("CLI process should run")
}

fn run_artifact(input: &Path, output: &Path) -> Output {
    run_artifact_with_columns(input, output, "time_s", "concentration_mol_l")
}

fn run_artifact_with_columns(
    input: &Path,
    output: &Path,
    time_column: &str,
    concentration_column: &str,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["kinetics", "artifact", "--input"])
        .arg(input)
        .args(["--time-column", time_column])
        .args(["--concentration-column", concentration_column])
        .arg("--output")
        .arg(output)
        .output()
        .expect("CLI process should run")
}

fn run_analyze_json(input: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["kinetics", "analyze", "--input"])
        .arg(input)
        .args([
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--json",
        ])
        .output()
        .expect("CLI process should run")
}

fn output_text(output: Output) -> (std::process::ExitStatus, String, String) {
    let status = output.status;
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    (status, stdout, stderr)
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
        fs::remove_file(file).expect("test-owned file cleanup should succeed");
    }
    fs::remove_dir(directory).expect("test-owned empty directory cleanup should succeed");
}

fn cleanup_empty_directory(directory: &Path) {
    assert!(directory_paths(directory).is_empty());
    fs::remove_dir(directory).expect("test-owned empty directory cleanup should succeed");
}

fn atomic_temp_path(output: &Path) -> PathBuf {
    let mut name = output
        .file_name()
        .expect("output should have a file name")
        .to_os_string();
    name.push(".atomic-write.tmp");
    output.with_file_name(name)
}

fn assert_failure(output: Output, expected_stderr: &str) -> String {
    let (status, stdout, stderr) = output_text(output);
    assert!(!status.success(), "command unexpectedly succeeded");
    assert_eq!(stdout, "");
    assert!(
        stderr.contains(expected_stderr),
        "stderr did not contain {expected_stderr:?}: {stderr}"
    );
    assert!(!stderr.contains("panic") && !stderr.contains("backtrace"));
    stderr
}

fn parse_envelope(path: &Path) -> (Vec<u8>, Value) {
    let bytes = fs::read(path).expect("artifact output should be readable");
    let value = serde_json::from_slice(&bytes).expect("artifact output should be valid JSON");
    (bytes, value)
}

#[test]
fn exact_artifact_help_forms_succeed_without_io() {
    let directory = create_test_directory("help");
    for flag in ["--help", "-h"] {
        let (status, stdout, stderr) =
            output_text(run_args(&directory, &["kinetics", "artifact", flag]));
        assert!(status.success());
        assert_eq!(stderr, "");
        assert!(stdout.contains("deepseek-science kinetics artifact \\"));
        assert!(stdout.contains("regular simple numeric UTF-8 CSV"));
        assert!(stdout.contains("16 MiB"));
        assert!(stdout.contains("UTF-8 without a BOM"));
        assert!(stdout.contains(".json"));
        assert!(stdout.contains("single envelope"));
        assert!(stdout.contains("not overwritten"));
        assert!(stdout.contains("payload sidecar"));
        assert!(stdout.contains("stderr"));
    }
    cleanup_empty_directory(&directory);
}

#[test]
fn mixed_help_unknown_positionals_missing_values_and_forbidden_flags_are_rejected() {
    let directory = create_test_directory("syntax");
    for args in [
        vec!["kinetics", "artifact", "--help", "--input", "input.csv"],
        vec!["kinetics", "artifact", "-h", "--output", "output.json"],
    ] {
        assert_failure(run_args(&directory, &args), "unknown argument");
    }
    assert_failure(
        run_args(&directory, &["kinetics", "artifact", "--unknown"]),
        "unknown argument --unknown",
    );
    assert_failure(
        run_args(&directory, &["kinetics", "artifact", "unexpected"]),
        "unexpected positional argument unexpected",
    );
    for option in [
        "--input",
        "--time-column",
        "--concentration-column",
        "--output",
    ] {
        assert_failure(
            run_args(&directory, &["kinetics", "artifact", option]),
            &format!("missing value for {option}"),
        );
        assert_failure(
            run_args(&directory, &["kinetics", "artifact", option, ""]),
            &format!("missing value for {option}"),
        );
    }
    for option in [
        "--json",
        "--manifest",
        "--manifest-output",
        "--payload-output",
        "--artifact-id",
        "--project",
        "--save-run",
        "--force",
        "--overwrite",
        "--format",
        "--model",
        "--explain",
        "--rag",
    ] {
        assert_failure(
            run_args(&directory, &["kinetics", "artifact", option]),
            &format!("unknown argument {option}"),
        );
    }
    cleanup_empty_directory(&directory);
}

#[test]
fn duplicate_and_missing_required_options_are_rejected() {
    let directory = create_test_directory("required");
    let valid = [
        "--input",
        "input.csv",
        "--time-column",
        "time",
        "--concentration-column",
        "concentration",
        "--output",
        "output.json",
    ];
    for option in [
        "--input",
        "--time-column",
        "--concentration-column",
        "--output",
    ] {
        let index = valid
            .iter()
            .position(|value| value == &option)
            .expect("option should be present");
        let mut duplicate = vec!["kinetics", "artifact"];
        duplicate.extend(valid);
        duplicate.extend([option, valid[index + 1]]);
        assert_failure(
            run_args(&directory, &duplicate),
            &format!("duplicate argument {option}"),
        );

        let mut missing = vec!["kinetics", "artifact"];
        missing.extend(
            valid
                .iter()
                .enumerate()
                .filter(|(position, _)| *position != index && *position != index + 1)
                .map(|(_, value)| *value),
        );
        assert_failure(
            run_args(&directory, &missing),
            &format!("missing required argument {option}"),
        );
    }
    cleanup_empty_directory(&directory);
}

#[test]
fn successful_artifact_has_exact_schema_hashes_payload_and_single_file_effect() {
    let directory = create_test_directory("success");
    let input = create_input(&directory, "input.csv", VALID_CSV);
    let output = directory.join("result.json");
    let (status, stdout, stderr) = output_text(run_artifact(&input, &output));

    assert!(status.success(), "artifact failed: {stderr}");
    assert_eq!(stdout, "kinetics artifact complete\n");
    assert_eq!(stderr, "");
    let (bytes, value) = parse_envelope(&output);
    let text = std::str::from_utf8(&bytes).expect("envelope should be UTF-8");
    assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(!text.contains('\r'));
    assert!(text.ends_with("}\n"));
    assert!(!text.ends_with("\n\n"));

    assert_eq!(value["schema_version"], "kinetics.artifact.v1");
    assert_eq!(value["artifact"]["kind"], "json");
    assert_eq!(
        value["artifact"]["title"],
        "Chemistry kinetics analysis result"
    );
    assert_eq!(
        value["artifact"]["content"]["media_type"],
        "application/json"
    );
    assert_eq!(
        value["artifact"]["content"]["schema_version"],
        "kinetics.analysis.v1"
    );
    assert_eq!(value["artifact"]["content"]["encoding"], "utf-8");
    assert_eq!(value["artifact"]["inputs"].as_array().unwrap().len(), 1);
    assert_eq!(value["artifact"]["inputs"][0]["role"], "source_csv");
    assert_eq!(
        value["artifact"]["provenance"]["workflow_id"],
        "chemistry.kinetics_csv"
    );
    assert_eq!(
        value["artifact"]["provenance"]["workflow_step"],
        "produce_analysis_result"
    );
    assert_eq!(
        value["artifact"]["provenance"]["producer_command"],
        "kinetics.artifact"
    );
    assert_eq!(
        value["artifact"]["provenance"]["producer_version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(value["artifact"]["review"]["status"], "passed");
    assert_eq!(value["artifact"]["review"]["finding_count"], 0);

    let payload = value["payload_utf8"]
        .as_str()
        .expect("payload should be a string");
    assert!(payload.ends_with('\n'));
    assert!(!payload.ends_with("\n\n"));
    let analyze = run_analyze_json(&input);
    assert!(analyze.status.success());
    assert_eq!(analyze.stderr, b"");
    assert_eq!(payload.as_bytes(), analyze.stdout);
    assert_eq!(
        value["artifact"]["content"]["byte_length"],
        u64::try_from(payload.len()).expect("payload length should fit")
    );
    assert_eq!(value["artifact"]["content"]["hash"]["algorithm"], "blake3");
    assert_eq!(
        value["artifact"]["content"]["hash"]["value"],
        hash_bytes(payload.as_bytes())
    );
    assert_eq!(
        value["artifact"]["inputs"][0]["byte_length"],
        u64::try_from(VALID_CSV.len()).expect("input length should fit")
    );
    assert_eq!(
        value["artifact"]["inputs"][0]["hash"]["algorithm"],
        "blake3"
    );
    assert_eq!(
        value["artifact"]["inputs"][0]["hash"]["value"],
        hash_bytes(VALID_CSV)
    );

    let artifact_text = serde_json::to_string(&value["artifact"]).unwrap();
    for prohibited in [
        "artifact_id",
        "run_id",
        "project_id",
        "timestamp",
        "model",
        "rag",
        "retrieval",
        ".atomic-write.tmp",
    ] {
        assert!(!artifact_text.to_lowercase().contains(prohibited));
    }
    assert!(!artifact_text.contains(output.to_string_lossy().as_ref()));
    assert!(!artifact_text.contains(input.to_string_lossy().as_ref()));
    assert!(payload.contains(input.to_string_lossy().as_ref()));
    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &output])
    );
    assert!(!atomic_temp_path(&output).exists());
    cleanup_files(&directory, &[&input, &output]);
}

#[test]
fn rejected_row_review_is_preserved_in_envelope_and_payload() {
    let directory = create_test_directory("review-warning");
    let input = create_input(&directory, "input.csv", WARNING_CSV);
    let output = directory.join("result.json");
    let (status, stdout, stderr) = output_text(run_artifact(&input, &output));
    assert!(status.success(), "artifact failed: {stderr}");
    assert_eq!(stdout, "kinetics artifact complete\n");
    assert_eq!(stderr, "");

    let (_, value) = parse_envelope(&output);
    let payload: Value = serde_json::from_str(value["payload_utf8"].as_str().unwrap()).unwrap();
    let payload_findings = payload["review"]["findings"].as_array().unwrap();
    assert_eq!(
        value["artifact"]["review"]["status"],
        "passed_with_warnings"
    );
    assert_eq!(
        value["artifact"]["review"]["finding_count"],
        u64::try_from(payload_findings.len()).expect("finding count should fit")
    );
    assert_eq!(payload["review"]["status"], "passed_with_warnings");
    cleanup_files(&directory, &[&input, &output]);
}

#[test]
fn fresh_output_paths_and_case_insensitive_extensions_are_byte_identical() {
    let directory = create_test_directory("determinism");
    let input = create_input(&directory, "input.csv", VALID_CSV);
    let outputs = [
        directory.join("first.json"),
        directory.join("second.JSON"),
        directory.join("third.JsOn"),
    ];
    for output in &outputs {
        let (status, stdout, stderr) = output_text(run_artifact(&input, output));
        assert!(status.success(), "artifact failed: {stderr}");
        assert_eq!(stdout, "kinetics artifact complete\n");
        assert_eq!(stderr, "");
        assert!(!atomic_temp_path(output).exists());
    }
    let first = fs::read(&outputs[0]).unwrap();
    assert_eq!(first, fs::read(&outputs[1]).unwrap());
    assert_eq!(first, fs::read(&outputs[2]).unwrap());
    cleanup_files(&directory, &[&input, &outputs[0], &outputs[1], &outputs[2]]);
}

#[test]
fn input_path_does_not_participate_in_source_hash() {
    let directory = create_test_directory("source-hash");
    let first_input = create_input(&directory, "first.csv", VALID_CSV);
    let second_input = create_input(&directory, "second.csv", VALID_CSV);
    let first_output = directory.join("first.json");
    let second_output = directory.join("second.json");
    assert!(run_artifact(&first_input, &first_output).status.success());
    assert!(run_artifact(&second_input, &second_output).status.success());

    let (_, first) = parse_envelope(&first_output);
    let (_, second) = parse_envelope(&second_output);
    assert_eq!(
        first["artifact"]["inputs"][0]["hash"],
        second["artifact"]["inputs"][0]["hash"]
    );
    assert_ne!(first["payload_utf8"], second["payload_utf8"]);
    cleanup_files(
        &directory,
        &[&first_input, &second_input, &first_output, &second_output],
    );
}

#[test]
fn create_new_refuses_existing_target_and_preserves_sentinel() {
    const SENTINEL: &[u8] = b"existing artifact sentinel\n";
    let directory = create_test_directory("existing-target");
    let input = create_input(&directory, "input.csv", VALID_CSV);
    let output = directory.join("result.json");
    fs::write(&output, SENTINEL).expect("sentinel should be written");

    let stderr = assert_failure(run_artifact(&input, &output), "already exists");
    assert!(!stderr.contains(".atomic-write.tmp"));
    assert_eq!(fs::read(&output).unwrap(), SENTINEL);
    assert!(!atomic_temp_path(&output).exists());
    cleanup_files(&directory, &[&input, &output]);
}

#[test]
fn lexical_equality_and_json_extension_are_checked_before_input_io() {
    let directory = create_test_directory("paths");
    let same = create_input(&directory, "same.json", VALID_CSV);
    assert_failure(
        run_artifact(&same, &same),
        "input and output paths must be different",
    );
    assert_eq!(fs::read(&same).unwrap(), VALID_CSV);

    for output in ["result", "result.svg", "result.json.txt", "result.json/"] {
        let stderr = assert_failure(
            run_args(
                &directory,
                &[
                    "kinetics",
                    "artifact",
                    "--input",
                    "missing.csv",
                    "--time-column",
                    "time",
                    "--concentration-column",
                    "concentration",
                    "--output",
                    output,
                ],
            ),
            if output.ends_with('/') {
                "must include a file name"
            } else {
                ".json extension"
            },
        );
        assert!(!stderr.contains("could not open"));
    }
    cleanup_files(&directory, &[&same]);
}

#[test]
fn missing_or_invalid_parent_is_not_created_or_modified() {
    let directory = create_test_directory("parent");
    let input = create_input(&directory, "input.csv", VALID_CSV);
    let missing_parent = directory.join("missing");
    let missing_output = missing_parent.join("result.json");
    assert_failure(
        run_artifact(&input, &missing_output),
        "does not exist or is not a directory",
    );
    assert!(!missing_parent.exists());

    let invalid_parent = directory.join("not-a-directory");
    const PARENT_SENTINEL: &[u8] = b"parent sentinel\n";
    fs::write(&invalid_parent, PARENT_SENTINEL).expect("parent sentinel should be written");
    assert_failure(
        run_artifact(&input, &invalid_parent.join("result.json")),
        "does not exist or is not a directory",
    );
    assert_eq!(fs::read(&invalid_parent).unwrap(), PARENT_SENTINEL);
    cleanup_files(&directory, &[&input, &invalid_parent]);
}

#[test]
fn invalid_inputs_fail_before_publication() {
    let directory = create_test_directory("invalid-input");
    let input = create_input(
        &directory,
        "input.csv",
        &[b"\xef\xbb\xbf".as_slice(), VALID_CSV].concat(),
    );
    let output = directory.join("result.json");

    assert_failure(run_artifact(&input, &output), "UTF-8 without a BOM");
    assert!(!output.exists());
    fs::write(&input, b"time_s,concentration_mol_l\n0,1\n1,\xff\n")
        .expect("invalid UTF-8 should be written");
    assert_failure(
        run_artifact(&input, &output),
        "not valid UTF-8 at byte offset",
    );
    assert!(!output.exists());
    fs::write(&input, b"time_s,concentration_mol_l\nnot,numeric\n")
        .expect("invalid CSV should be written");
    assert_failure(run_artifact(&input, &output), "invalid CSV");
    assert!(!output.exists());
    fs::write(&input, VALID_CSV).expect("valid CSV should be restored");
    assert_failure(
        run_artifact_with_columns(&input, &output, "missing", "concentration_mol_l"),
        "time column",
    );
    assert!(!output.exists());
    assert_failure(
        run_artifact_with_columns(&input, &output, "time_s", "missing"),
        "concentration column",
    );
    assert!(!output.exists());
    fs::write(&input, b"time_s,concentration_mol_l\n0,1\n1,0\n")
        .expect("too-few-points input should be written");
    assert_failure(
        run_artifact(&input, &output),
        "not enough valid kinetics points",
    );
    assert!(!output.exists());
    assert!(!atomic_temp_path(&output).exists());
    cleanup_files(&directory, &[&input]);
}

#[test]
fn missing_and_non_regular_inputs_are_rejected() {
    let directory = create_test_directory("input-paths");
    let output = directory.join("result.json");
    let missing = directory.join("missing.csv");
    assert_failure(run_artifact(&missing, &output), "could not open input file");
    assert!(!output.exists());

    let input_directory = directory.join("input-directory");
    fs::create_dir(&input_directory).expect("test input directory should be created");
    assert_failure(
        run_artifact(&input_directory, &output),
        "must refer to a regular file",
    );
    assert!(!output.exists());
    fs::remove_dir(&input_directory).expect("test-owned input directory cleanup should succeed");
    cleanup_empty_directory(&directory);
}

#[test]
fn sparse_over_limit_input_is_rejected_without_output() {
    let directory = create_test_directory("input-limit");
    let input = directory.join("input.csv");
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&input)
        .expect("sparse input should be created");
    file.set_len(MAX_INPUT_BYTES + 1)
        .expect("sparse input length should be set");
    drop(file);
    let output = directory.join("result.json");

    assert_failure(
        run_artifact(&input, &output),
        "exceeds the fixed 16 MiB limit",
    );
    assert!(!output.exists());
    assert!(!atomic_temp_path(&output).exists());
    cleanup_files(&directory, &[&input]);
}

#[test]
fn stale_temporary_sibling_is_preserved_and_not_exposed() {
    let directory = create_test_directory("publication-failure");
    let input = create_input(&directory, "input.csv", VALID_CSV);
    let output = directory.join("result.json");
    let temp = atomic_temp_path(&output);
    const TEMP_SENTINEL: &[u8] = b"unowned stale sibling\n";
    fs::write(&temp, TEMP_SENTINEL).expect("stale sibling should be written");

    let stderr = assert_failure(
        run_artifact(&input, &output),
        "may exist, inspect it before retrying",
    );
    assert!(!stderr.contains(".atomic-write.tmp"));
    assert!(!output.exists());
    assert_eq!(fs::read(&temp).unwrap(), TEMP_SENTINEL);
    cleanup_files(&directory, &[&input, &temp]);
}
