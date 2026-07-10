use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_kinetics_analyze(fixture_name: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["kinetics", "analyze", "--input"])
        .arg(fixture_path(fixture_name))
        .args([
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ])
        .output()
        .expect("CLI process should run")
}

fn run_kinetics_analyze_json(fixture_name: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["kinetics", "analyze", "--input"])
        .arg(fixture_path(fixture_name))
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

fn run_kinetics_analyze_with_output(
    fixture_name: &str,
    output_path: &Path,
    json_stdout: bool,
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_deepseek-science"));
    command
        .args(["kinetics", "analyze", "--input"])
        .arg(fixture_path(fixture_name))
        .args([
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ]);
    if json_stdout {
        command.arg("--json");
    }

    command
        .arg("--output")
        .arg(output_path)
        .output()
        .expect("CLI process should run")
}

fn create_test_directory(label: &str) -> PathBuf {
    let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let target_tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .canonicalize()
        .expect("Cargo target temp directory should be resolvable");
    let directory = target_tmp.join(format!(
        "deepseek-science-cli-{label}-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("test directory should be unique");

    directory
}

fn directory_paths(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .expect("test directory should be readable")
        .map(|entry| entry.expect("test entry should be readable").path())
        .collect()
}

fn run_kinetics_analyze_help() -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["kinetics", "analyze", "--help"])
        .output()
        .expect("CLI process should run")
}

fn output_text(output: std::process::Output) -> (std::process::ExitStatus, String, String) {
    let status = output.status;
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");

    (status, stdout, stderr)
}

fn assert_user_error_without_success_summary(stdout: &str, stderr: &str) {
    let combined = format!("{stdout}\n{stderr}").to_lowercase();

    assert!(
        stdout.is_empty() || !stdout.contains("DeepSeek_Science kinetics analyze"),
        "stdout should not contain a success summary: {stdout}"
    );
    assert!(
        stderr.contains("error:"),
        "stderr should contain an error: {stderr}"
    );
    assert!(
        !combined.contains("panic") && !combined.contains("backtrace"),
        "error output should not expose panic/backtrace wording: {combined}"
    );
}

#[test]
fn kinetics_analyze_process_help_prints_usage_to_stdout() {
    let (status, stdout, stderr) = output_text(run_kinetics_analyze_help());

    assert!(
        status.success(),
        "expected help success, stderr:\n{stderr}\nstdout:\n{stdout}"
    );
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--input <path>"));
    assert!(stdout.contains("--time-column <column>"));
    assert!(stdout.contains("--concentration-column <column>"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--output <path>"));
    assert!(stdout.contains("Existing targets are not overwritten"));
    assert!(stdout.contains("parent directories are not created"));
    assert_eq!(stderr, "");
}

#[test]
fn kinetics_analyze_process_succeeds_with_project_fixture() {
    let (status, stdout, stderr) = output_text(run_kinetics_analyze("kinetics_success.csv"));

    assert!(
        status.success(),
        "expected success, stderr:\n{stderr}\nstdout:\n{stdout}"
    );
    assert!(stdout.contains("first_order.k:"));
    assert!(stdout.contains("first_order.r_squared:"));
    assert!(stdout.contains("second_order.k:"));
    assert!(stdout.contains("second_order.r_squared:"));
    assert!(stdout.contains("preferred_model:"));
    assert!(stdout.contains("Preferred by MVP r_squared heuristic"));
    assert!(stdout.contains("review_status:"));
    assert!(
        !stderr.contains("error:"),
        "stderr should not contain an error: {stderr}"
    );
    assert!(!stdout.contains("definitive"));
    assert!(!stdout.contains("true model"));
    assert!(!stdout.contains("proved"));
    assert!(!stdout.contains("proof"));
}

#[test]
fn kinetics_analyze_process_json_success_outputs_deterministic_json() {
    let fixture = fixture_path("kinetics_success.csv");
    let (status, stdout, stderr) = output_text(run_kinetics_analyze_json("kinetics_success.csv"));

    assert!(
        status.success(),
        "expected success, stderr:\n{stderr}\nstdout:\n{stdout}"
    );
    assert_eq!(stderr, "");

    let value: Value = serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    let expected_input_path = fixture.to_string_lossy().to_string();

    assert_eq!(value["schema_version"], "kinetics.analysis.v1");
    assert_eq!(value["command"], "kinetics.analyze");
    assert_eq!(value["input"]["path"], expected_input_path);
    assert_eq!(value["columns"]["time"], "time_s");
    assert_eq!(value["columns"]["concentration"], "concentration_mol_l");
    assert!(value["counts"]["valid_points"].is_number());
    assert!(value["counts"]["rejected_rows"].is_number());
    assert!(value["fits"]["first_order"].is_object());
    assert!(value["fits"]["second_order"].is_object());
    assert_eq!(
        value["comparison"]["basis"],
        "finite_r_squared_mvp_heuristic"
    );
    assert!(value["comparison"]["caution"]
        .as_str()
        .expect("caution should be a string")
        .contains("mvp_r_squared_heuristic"));
    assert!(value["review"]["status"].is_string());

    let lower_stdout = stdout.to_lowercase();
    assert!(!lower_stdout.contains("definitive"));
    assert!(!lower_stdout.contains("true model"));
    assert!(!lower_stdout.contains("proved"));
    assert!(!lower_stdout.contains("proof"));
    assert!(!lower_stdout.contains("final reaction order"));
}

#[test]
fn kinetics_analyze_output_keeps_text_stdout_and_saves_json() {
    let directory = create_test_directory("text-output");
    let target = directory.join("result.json");
    let (status, stdout, stderr) = output_text(run_kinetics_analyze_with_output(
        "kinetics_success.csv",
        &target,
        false,
    ));

    assert!(status.success(), "expected success, stderr: {stderr}");
    assert!(stdout.contains("DeepSeek_Science kinetics analyze"));
    assert!(stdout.contains("first_order.k:"));
    assert_eq!(stderr, "");

    let saved = fs::read_to_string(&target).expect("saved JSON should be readable");
    let value: Value = serde_json::from_str(&saved).expect("saved output should be valid JSON");
    assert_eq!(value["schema_version"], "kinetics.analysis.v1");
    assert_eq!(value["command"], "kinetics.analyze");
    assert_eq!(directory_paths(&directory), vec![target.clone()]);

    fs::remove_file(&target).expect("target cleanup should succeed");
    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn kinetics_analyze_json_stdout_matches_saved_bytes() {
    let directory = create_test_directory("json-output");
    let target = directory.join("result.json");
    let output = run_kinetics_analyze_with_output("kinetics_success.csv", &target, true);

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"");
    assert_eq!(
        output.stdout,
        fs::read(&target).expect("saved JSON should be readable")
    );
    assert_eq!(directory_paths(&directory), vec![target.clone()]);

    fs::remove_file(&target).expect("target cleanup should succeed");
    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn kinetics_analyze_output_refuses_existing_target_without_changes() {
    const SENTINEL: &[u8] = b"existing\n";

    let directory = create_test_directory("existing-output");
    let target = directory.join("result.json");
    fs::write(&target, SENTINEL).expect("sentinel setup should succeed");
    let (status, stdout, stderr) = output_text(run_kinetics_analyze_with_output(
        "kinetics_success.csv",
        &target,
        false,
    ));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("could not write output file"));
    assert!(stderr.contains("target already exists"));
    assert_eq!(
        fs::read(&target).expect("target should be readable"),
        SENTINEL
    );
    assert_eq!(directory_paths(&directory), vec![target.clone()]);

    fs::remove_file(&target).expect("target cleanup should succeed");
    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn kinetics_analyze_output_does_not_create_missing_parent() {
    let directory = create_test_directory("missing-parent");
    let missing_parent = directory.join("missing");
    let target = missing_parent.join("result.json");
    let (status, stdout, stderr) = output_text(run_kinetics_analyze_with_output(
        "kinetics_success.csv",
        &target,
        false,
    ));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("parent directory is missing"));
    assert!(!missing_parent.exists());
    assert!(!target.exists());
    assert!(directory_paths(&directory).is_empty());

    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn kinetics_analyze_failure_creates_no_output_file() {
    let directory = create_test_directory("analysis-failure");
    let target = directory.join("result.json");
    let (status, stdout, stderr) = output_text(run_kinetics_analyze_with_output(
        "kinetics_invalid_csv.csv",
        &target,
        false,
    ));

    assert!(!status.success());
    assert_eq!(stdout, "");
    assert!(stderr.contains("invalid CSV"));
    assert!(!target.exists());
    assert!(directory_paths(&directory).is_empty());

    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn kinetics_analyze_process_reports_missing_file_as_user_error() {
    let (status, stdout, stderr) = output_text(run_kinetics_analyze("missing_kinetics.csv"));

    assert!(!status.success());
    assert!(
        stdout.is_empty(),
        "stdout should be empty on user error: {stdout}"
    );
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("could not read input file"));
}

#[test]
fn kinetics_analyze_process_reports_invalid_csv_as_user_error() {
    let (status, stdout, stderr) = output_text(run_kinetics_analyze("kinetics_invalid_csv.csv"));

    assert!(!status.success());
    assert_user_error_without_success_summary(&stdout, &stderr);
    assert!(
        stderr.contains("invalid CSV") || stderr.contains("invalid float"),
        "stderr should mention CSV parsing failure: {stderr}"
    );
}

#[test]
fn kinetics_analyze_process_reports_missing_time_column_as_user_error() {
    let (status, stdout, stderr) =
        output_text(run_kinetics_analyze("kinetics_missing_time_column.csv"));
    let stderr_lower = stderr.to_lowercase();

    assert!(!status.success());
    assert_user_error_without_success_summary(&stdout, &stderr);
    assert!(
        stderr_lower.contains("missing")
            && stderr_lower.contains("time")
            && stderr_lower.contains("column"),
        "stderr should mention the missing time column: {stderr}"
    );
}

#[test]
fn kinetics_analyze_process_reports_missing_concentration_column_as_user_error() {
    let (status, stdout, stderr) = output_text(run_kinetics_analyze(
        "kinetics_missing_concentration_column.csv",
    ));
    let stderr_lower = stderr.to_lowercase();

    assert!(!status.success());
    assert_user_error_without_success_summary(&stdout, &stderr);
    assert!(
        stderr_lower.contains("missing")
            && stderr_lower.contains("concentration")
            && stderr_lower.contains("column"),
        "stderr should mention the missing concentration column: {stderr}"
    );
}
