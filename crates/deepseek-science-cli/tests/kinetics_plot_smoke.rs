use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const ROOT_LINE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 960 640\" width=\"960\" height=\"640\" role=\"img\" aria-labelledby=\"plot-title plot-desc\" font-family=\"system-ui, sans-serif\">";
const VALID_CSV: &[u8] = b"time_s,concentration_mol_l\n0,1\n1,0.8\n2,0.6\n";

static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn create_test_directory(label: &str) -> PathBuf {
    let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let target_tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .canonicalize()
        .expect("Cargo target temp directory should be resolvable");
    let directory = target_tmp.join(format!(
        "deepseek-science-kinetics-plot-{label}-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("test directory should be unique");
    directory
}

fn create_input(directory: &Path, bytes: &[u8]) -> PathBuf {
    let input = directory.join("input.csv");
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

fn run_plot(input: &Path, output: &Path) -> Output {
    run_plot_with_columns(input, output, "time_s", "concentration_mol_l")
}

fn run_plot_with_columns(
    input: &Path,
    output: &Path,
    time_column: &str,
    concentration_column: &str,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args(["kinetics", "plot", "--input"])
        .arg(input)
        .args(["--time-column", time_column])
        .args(["--concentration-column", concentration_column])
        .arg("--output")
        .arg(output)
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
    fs::remove_dir(directory).expect("test-owned directory cleanup should succeed");
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

#[test]
fn exact_plot_help_forms_succeed_without_io() {
    let directory = create_test_directory("help");
    for flag in ["--help", "-h"] {
        let (status, stdout, stderr) =
            output_text(run_args(&directory, &["kinetics", "plot", flag]));
        assert!(status.success());
        assert_eq!(stderr, "");
        assert!(stdout.contains("Usage:"));
        assert!(stdout.contains("deepseek-science kinetics plot \\"));
        assert!(stdout.contains("--input <path> \\"));
        assert!(stdout.contains("--time-column <column> \\"));
        assert!(stdout.contains("--concentration-column <column> \\"));
        assert!(stdout.contains("--output <path.svg>"));
        assert!(stdout.contains("simple numeric UTF-8 CSV"));
        assert!(stdout.contains("16 MiB"));
        assert!(stdout.contains("Existing targets are not overwritten"));
    }
    assert!(directory_paths(&directory).is_empty());
    fs::remove_dir(&directory).expect("test-owned directory cleanup should succeed");
}

#[test]
fn help_mixed_with_arguments_and_invalid_syntax_are_rejected() {
    let directory = create_test_directory("syntax");
    for args in [
        vec!["kinetics", "plot", "--help", "--input", "input.csv"],
        vec!["kinetics", "plot", "-h", "--output", "output.svg"],
    ] {
        assert_failure(run_args(&directory, &args), "unknown argument");
    }

    assert_failure(
        run_args(&directory, &["kinetics", "plot", "--unknown"]),
        "unknown argument --unknown",
    );
    assert_failure(
        run_args(&directory, &["kinetics", "plot", "unexpected"]),
        "unexpected positional argument unexpected",
    );
    for option in [
        "--input",
        "--time-column",
        "--concentration-column",
        "--output",
    ] {
        assert_failure(
            run_args(&directory, &["kinetics", "plot", option, ""]),
            &format!("missing value for {option}"),
        );
    }

    for option in [
        "--input",
        "--time-column",
        "--concentration-column",
        "--output",
    ] {
        assert_failure(
            run_args(&directory, &["kinetics", "plot", option]),
            &format!("missing value for {option}"),
        );
    }
    assert!(directory_paths(&directory).is_empty());
    fs::remove_dir(&directory).expect("test-owned directory cleanup should succeed");
}

#[test]
fn duplicate_and_missing_required_options_are_rejected() {
    let directory = create_test_directory("required-options");
    for (option, args) in [
        (
            "--input",
            vec![
                "kinetics",
                "plot",
                "--input",
                "one.csv",
                "--input",
                "two.csv",
                "--time-column",
                "time",
                "--concentration-column",
                "concentration",
                "--output",
                "out.svg",
            ],
        ),
        (
            "--time-column",
            vec![
                "kinetics",
                "plot",
                "--input",
                "one.csv",
                "--time-column",
                "one",
                "--time-column",
                "two",
                "--concentration-column",
                "concentration",
                "--output",
                "out.svg",
            ],
        ),
        (
            "--concentration-column",
            vec![
                "kinetics",
                "plot",
                "--input",
                "one.csv",
                "--time-column",
                "time",
                "--concentration-column",
                "one",
                "--concentration-column",
                "two",
                "--output",
                "out.svg",
            ],
        ),
        (
            "--output",
            vec![
                "kinetics",
                "plot",
                "--input",
                "one.csv",
                "--time-column",
                "time",
                "--concentration-column",
                "concentration",
                "--output",
                "one.svg",
                "--output",
                "two.svg",
            ],
        ),
    ] {
        assert_failure(
            run_args(&directory, &args),
            &format!("duplicate argument {option}"),
        );
    }

    for (missing, args) in [
        (
            "--input",
            vec![
                "kinetics",
                "plot",
                "--time-column",
                "time",
                "--concentration-column",
                "concentration",
                "--output",
                "out.svg",
            ],
        ),
        (
            "--time-column",
            vec![
                "kinetics",
                "plot",
                "--input",
                "input.csv",
                "--concentration-column",
                "concentration",
                "--output",
                "out.svg",
            ],
        ),
        (
            "--concentration-column",
            vec![
                "kinetics",
                "plot",
                "--input",
                "input.csv",
                "--time-column",
                "time",
                "--output",
                "out.svg",
            ],
        ),
        (
            "--output",
            vec![
                "kinetics",
                "plot",
                "--input",
                "input.csv",
                "--time-column",
                "time",
                "--concentration-column",
                "concentration",
            ],
        ),
    ] {
        assert_failure(
            run_args(&directory, &args),
            &format!("missing required argument {missing}"),
        );
    }
    assert!(directory_paths(&directory).is_empty());
    fs::remove_dir(&directory).expect("test-owned directory cleanup should succeed");
}

#[test]
fn lexical_equality_and_svg_extension_order_are_enforced() {
    let directory = create_test_directory("paths");
    let stderr = assert_failure(
        run_args(
            &directory,
            &[
                "kinetics",
                "plot",
                "--input",
                "same.csv",
                "--time-column",
                "time",
                "--concentration-column",
                "concentration",
                "--output",
                "same.csv",
            ],
        ),
        "input and output paths must be different",
    );
    assert!(!stderr.contains("could not open"));

    for output in [
        "output",
        "output.png",
        "output.json",
        "output.html",
        "output.svg.txt",
    ] {
        let stderr = assert_failure(
            run_args(
                &directory,
                &[
                    "kinetics",
                    "plot",
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
            ".svg extension",
        );
        assert!(!stderr.contains("could not open"));
    }
    assert_failure(
        run_args(
            &directory,
            &[
                "kinetics",
                "plot",
                "--input",
                "missing.csv",
                "--time-column",
                "time",
                "--concentration-column",
                "concentration",
                "--output",
                "output.svg/",
            ],
        ),
        "must include a file name",
    );
    assert!(directory_paths(&directory).is_empty());
    fs::remove_dir(&directory).expect("test-owned directory cleanup should succeed");
}

#[cfg(unix)]
#[test]
fn non_utf8_output_extension_is_rejected_without_panic() {
    use std::os::unix::ffi::OsStringExt;

    let output = Command::new(env!("CARGO_BIN_EXE_deepseek-science"))
        .args([
            OsString::from("kinetics"),
            OsString::from("plot"),
            OsString::from("--input"),
            OsString::from("missing.csv"),
            OsString::from("--time-column"),
            OsString::from("time"),
            OsString::from("--concentration-column"),
            OsString::from("concentration"),
            OsString::from("--output"),
            OsString::from_vec(b"output.\xff".to_vec()),
        ])
        .output()
        .expect("CLI process should run");
    assert_failure(output, "CLI arguments must be valid UTF-8");
}

#[test]
fn ascii_case_insensitive_svg_extensions_publish_successfully() {
    let directory = create_test_directory("extensions");
    let input = create_input(&directory, VALID_CSV);
    let mut outputs = Vec::new();
    for name in ["lower.svg", "upper.SVG", "mixed.SvG"] {
        let output = directory.join(name);
        let (status, stdout, stderr) = output_text(run_plot(&input, &output));
        assert!(status.success(), "plot failed: {stderr}");
        assert_eq!(stdout, "kinetics plot complete\n");
        assert_eq!(stderr, "");
        assert!(output.is_file());
        outputs.push(output);
    }
    let expected = sorted_paths(&[&input, &outputs[0], &outputs[1], &outputs[2]]);
    assert_eq!(directory_paths(&directory), expected);
    cleanup_files(&directory, &[&input, &outputs[0], &outputs[1], &outputs[2]]);
}

#[test]
fn invalid_input_forms_fail_before_publication() {
    let directory = create_test_directory("invalid-input");
    let input = create_input(&directory, b"\xff\xfe\x00");
    let output = directory.join("output.svg");

    assert_failure(run_plot(&input, &output), "not valid UTF-8");
    assert!(!output.exists());

    fs::write(&input, [b"\xef\xbb\xbf".as_slice(), VALID_CSV].concat())
        .expect("UTF-8 BOM test bytes should be written");
    assert_failure(run_plot(&input, &output), "UTF-8 without a BOM");
    assert!(!output.exists());

    fs::write(&input, b"\xff\xfet\x00i\x00m\x00e\x00\n\x00")
        .expect("UTF-16 test bytes should be written");
    assert_failure(run_plot(&input, &output), "not valid UTF-8");
    assert!(!output.exists());

    fs::write(&input, b"time_s,concentration_mol_l\nnot,numeric\n")
        .expect("invalid CSV should be written");
    assert_failure(run_plot(&input, &output), "invalid CSV");
    assert!(!output.exists());

    fs::write(&input, VALID_CSV).expect("valid CSV should be restored");
    assert_failure(
        run_plot_with_columns(&input, &output, "missing_time", "concentration_mol_l"),
        "time column",
    );
    assert!(!output.exists());
    assert_failure(
        run_plot_with_columns(&input, &output, "time_s", "missing_concentration"),
        "concentration column",
    );
    assert!(!output.exists());

    fs::write(&input, b"time_s,concentration_mol_l\n0,0\n1,-1\n2,0\n")
        .expect("rejected kinetics input should be written");
    assert_failure(run_plot(&input, &output), "kinetics analysis failed");
    assert!(!output.exists());

    fs::write(&input, b"time_s,concentration_mol_l\n1,1\n1,0.8\n1,0.6\n")
        .expect("analysis-failure input should be written");
    assert_failure(run_plot(&input, &output), "kinetics analysis failed");
    assert!(!output.exists());
    assert_eq!(directory_paths(&directory), vec![input.clone()]);
    cleanup_files(&directory, &[&input]);
}

#[test]
fn nonexistent_and_directory_inputs_are_rejected() {
    let directory = create_test_directory("input-paths");
    let output = directory.join("output.svg");
    let missing = directory.join("missing.csv");
    assert_failure(run_plot(&missing, &output), "could not open input file");
    assert!(!output.exists());

    let input_directory = directory.join("input-directory");
    fs::create_dir(&input_directory).expect("test input directory should be created");
    assert_failure(
        run_plot(&input_directory, &output),
        "must refer to a regular file",
    );
    assert!(!output.exists());
    assert_eq!(directory_paths(&directory), vec![input_directory.clone()]);
    fs::remove_dir(&input_directory).expect("test-owned input directory cleanup should succeed");
    fs::remove_dir(&directory).expect("test-owned directory cleanup should succeed");
}

#[test]
fn exact_limit_is_read_and_limit_plus_one_is_rejected() {
    let directory = create_test_directory("input-limit");
    let input = create_input(&directory, &vec![b'x'; MAX_INPUT_BYTES]);
    let output = directory.join("output.svg");

    let exact_stderr = assert_failure(run_plot(&input, &output), "invalid CSV");
    assert!(!exact_stderr.contains("exceeds the fixed 16 MiB"));
    assert!(!output.exists());

    fs::write(&input, vec![b'x'; MAX_INPUT_BYTES + 1]).expect("over-limit input should be written");
    assert_failure(run_plot(&input, &output), "exceeds the fixed 16 MiB");
    assert!(!output.exists());
    assert_eq!(directory_paths(&directory), vec![input.clone()]);
    cleanup_files(&directory, &[&input]);
}

#[test]
fn successful_plot_publishes_one_contract_compliant_svg() {
    let directory = create_test_directory("success");
    let input = create_input(&directory, VALID_CSV);
    let original_input = fs::read(&input).expect("input should be readable");
    let output = directory.join("result.svg");
    let (status, stdout, stderr) = output_text(run_plot(&input, &output));

    assert!(status.success(), "plot failed: {stderr}");
    assert_eq!(stdout, "kinetics plot complete\n");
    assert_eq!(stderr, "");
    let bytes = fs::read(&output).expect("SVG target should exist");
    let svg = std::str::from_utf8(&bytes).expect("SVG should be UTF-8");
    assert_eq!(svg.lines().next(), Some(ROOT_LINE));
    assert!(svg.contains("<title id=\"plot-title\">Kinetics concentration versus time</title>"));
    assert!(svg.contains("<desc id=\"plot-desc\">"));
    assert!(svg.contains("viewBox=\"0 0 960 640\" width=\"960\" height=\"640\""));
    assert!(svg.ends_with("</svg>\n"));
    assert!(!svg.ends_with("\n\n"));
    assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(!svg.contains('\r'));

    let observation_start = svg.find("<g id=\"observations\">").unwrap();
    let observation_end = svg.find("<g id=\"axis-labels\"").unwrap();
    assert_eq!(
        svg[observation_start..observation_end]
            .matches("<circle ")
            .count(),
        3
    );
    let first_start = svg.find("<g id=\"first-order-curves\">").unwrap();
    let second_start = svg.find("<g id=\"second-order-curves\">").unwrap();
    assert!(svg[first_start..second_start].contains("<polyline "));
    assert!(svg[second_start..observation_start].contains("<polyline "));
    assert!(svg.contains("MVP heuristic preference:"));
    assert!(svg.contains("accepted observations: 3"));
    assert!(svg.contains("rejected rows: 0"));
    assert!(svg.contains("visualization warnings: 0"));
    assert!(!svg.contains(input.to_string_lossy().as_ref()));
    assert!(!svg.contains(output.to_string_lossy().as_ref()));
    assert!(!svg.contains(".atomic-write.tmp"));
    assert!(!directory.join("result.json").exists());
    assert_eq!(fs::read(&input).unwrap(), original_input);
    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &output])
    );
    assert!(!atomic_temp_path(&output).exists());
    cleanup_files(&directory, &[&input, &output]);
}

#[test]
fn create_new_refuses_existing_target_and_preserves_input_and_sentinel() {
    const SENTINEL: &[u8] = b"existing SVG sentinel\n";

    let directory = create_test_directory("existing-target");
    let input = create_input(&directory, VALID_CSV);
    let output = directory.join("result.svg");
    fs::write(&output, SENTINEL).expect("sentinel should be written");
    let stderr = assert_failure(run_plot(&input, &output), "already exists");

    assert!(!stderr.contains(".atomic-write.tmp"));
    assert_eq!(fs::read(&output).unwrap(), SENTINEL);
    assert_eq!(fs::read(&input).unwrap(), VALID_CSV);
    assert!(!atomic_temp_path(&output).exists());
    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &output])
    );
    cleanup_files(&directory, &[&input, &output]);
}

#[test]
fn missing_or_invalid_parent_is_not_created_or_modified() {
    let directory = create_test_directory("parent");
    let input = create_input(&directory, VALID_CSV);
    let missing_parent = directory.join("missing");
    let missing_output = missing_parent.join("result.svg");
    assert_failure(
        run_plot(&input, &missing_output),
        "does not exist or is not a directory",
    );
    assert!(!missing_parent.exists());
    assert!(!missing_output.exists());

    let invalid_parent = directory.join("not-a-directory");
    const PARENT_SENTINEL: &[u8] = b"parent sentinel\n";
    fs::write(&invalid_parent, PARENT_SENTINEL).expect("parent sentinel should be written");
    let invalid_output = invalid_parent.join("result.svg");
    assert_failure(
        run_plot(&input, &invalid_output),
        "does not exist or is not a directory",
    );
    assert_eq!(fs::read(&invalid_parent).unwrap(), PARENT_SENTINEL);
    assert!(!invalid_output.exists());
    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &invalid_parent])
    );
    cleanup_files(&directory, &[&input, &invalid_parent]);
}

#[test]
fn uncertain_publication_failure_is_conservative_and_hides_temp_path() {
    let directory = create_test_directory("publication-failure");
    let input = create_input(&directory, VALID_CSV);
    let output = directory.join("result.svg");
    let temp = atomic_temp_path(&output);
    const TEMP_SENTINEL: &[u8] = b"unowned stale sibling\n";
    fs::write(&temp, TEMP_SENTINEL).expect("stale sibling should be written");

    let stderr = assert_failure(
        run_plot(&input, &output),
        "may exist, inspect it before retrying",
    );
    assert!(!stderr.contains(".atomic-write.tmp"));
    assert!(!output.exists());
    assert_eq!(fs::read(&temp).unwrap(), TEMP_SENTINEL);
    assert_eq!(directory_paths(&directory), sorted_paths(&[&input, &temp]));
    cleanup_files(&directory, &[&input, &temp]);
}

#[test]
fn distinct_targets_are_byte_identical_and_repeat_refuses_overwrite() {
    let directory = create_test_directory("determinism");
    let input = create_input(&directory, VALID_CSV);
    let first = directory.join("first.svg");
    let second = directory.join("second.SVG");

    for target in [&first, &second] {
        let (status, stdout, stderr) = output_text(run_plot(&input, target));
        assert!(status.success(), "plot failed: {stderr}");
        assert_eq!(stdout, "kinetics plot complete\n");
        assert_eq!(stderr, "");
        assert!(!atomic_temp_path(target).exists());
    }
    let first_bytes = fs::read(&first).unwrap();
    assert_eq!(first_bytes, fs::read(&second).unwrap());

    assert_failure(run_plot(&input, &first), "already exists");
    assert_eq!(fs::read(&first).unwrap(), first_bytes);
    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &first, &second])
    );
    cleanup_files(&directory, &[&input, &first, &second]);
}

#[test]
fn existing_commands_remain_compatible() {
    let directory = create_test_directory("compatibility");
    let input = create_input(&directory, VALID_CSV);

    let (status, stdout, stderr) = output_text(run_args(
        &directory,
        &[
            "kinetics",
            "analyze",
            "--input",
            "input.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
        ],
    ));
    assert!(status.success(), "analyze text failed: {stderr}");
    assert!(stdout.contains("DeepSeek_Science kinetics analyze"));
    assert!(stdout.contains("preferred_note: Preferred by MVP r_squared heuristic"));
    assert_eq!(stderr, "");

    let (status, stdout, stderr) = output_text(run_args(
        &directory,
        &[
            "kinetics",
            "analyze",
            "--input",
            "input.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--json",
        ],
    ));
    assert!(status.success(), "analyze JSON failed: {stderr}");
    let json: Value = serde_json::from_str(&stdout).expect("JSON stdout should parse");
    assert_eq!(json["schema_version"], "kinetics.analysis.v1");
    assert_eq!(json["command"], "kinetics.analyze");
    assert_eq!(stderr, "");

    let analysis_output = directory.join("analysis.json");
    let (status, stdout, stderr) = output_text(run_args(
        &directory,
        &[
            "kinetics",
            "analyze",
            "--input",
            "input.csv",
            "--time-column",
            "time_s",
            "--concentration-column",
            "concentration_mol_l",
            "--output",
            "analysis.json",
        ],
    ));
    assert!(status.success(), "analyze output failed: {stderr}");
    assert!(stdout.contains("DeepSeek_Science kinetics analyze"));
    let saved: Value = serde_json::from_slice(&fs::read(&analysis_output).unwrap()).unwrap();
    assert_eq!(saved["schema_version"], "kinetics.analysis.v1");

    let (status, stdout, stderr) = output_text(run_args(
        &directory,
        &["data", "inspect", "--input", "input.csv"],
    ));
    assert!(status.success(), "data inspect failed: {stderr}");
    assert!(stdout.contains("inspection_status:"));
    assert_eq!(stderr, "");

    let tab_input = directory.join("input.tsv");
    let converted = directory.join("converted.csv");
    fs::write(&tab_input, b"time_s\tconcentration_mol_l\n0\t1\n1\t0.8\n")
        .expect("tab input should be written");
    let (status, stdout, stderr) = output_text(run_args(
        &directory,
        &[
            "data",
            "convert",
            "--input",
            "input.tsv",
            "--output",
            "converted.csv",
        ],
    ));
    assert!(status.success(), "data convert failed: {stderr}");
    assert!(stdout.starts_with("conversion_status: complete\n"));
    assert_eq!(stderr, "");
    assert_eq!(
        fs::read(&converted).unwrap(),
        b"time_s,concentration_mol_l\n0,1\n1,0.8\n"
    );

    assert_eq!(
        directory_paths(&directory),
        sorted_paths(&[&input, &analysis_output, &tab_input, &converted])
    );
    cleanup_files(
        &directory,
        &[&input, &analysis_output, &tab_input, &converted],
    );
}
