use std::path::PathBuf;
use std::process::Command;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn kinetics_analyze_process_succeeds_with_project_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["kinetics", "analyze", "--input"])
        .arg(fixture_path("kinetics_success.csv"))
        .args([
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ])
        .output()
        .expect("CLI process should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");

    assert!(
        output.status.success(),
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
fn kinetics_analyze_process_reports_missing_file_as_user_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["kinetics", "analyze", "--input"])
        .arg(fixture_path("missing_kinetics.csv"))
        .args([
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ])
        .output()
        .expect("CLI process should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");

    assert!(!output.status.success());
    assert!(
        stdout.is_empty(),
        "stdout should be empty on user error: {stdout}"
    );
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("could not read input file"));
}
