use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

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
