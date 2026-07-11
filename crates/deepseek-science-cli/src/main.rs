#![forbid(unsafe_code)]
//! Binary entry point for the Phase 1 headless CLI.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args = std::env::args_os()
        .map(|argument| argument.into_string())
        .collect::<Result<Vec<_>, _>>();
    let output = match args {
        Ok(args) => deepseek_science_cli::run_cli(args),
        Err(_) => deepseek_science_cli::CliOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error: CLI arguments must be valid UTF-8\n".to_string(),
        },
    };
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);

    if output.exit_code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(output.exit_code as u8)
    }
}
