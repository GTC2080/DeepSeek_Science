#![forbid(unsafe_code)]
//! Binary entry point for the Phase 1 headless CLI.

use std::process::ExitCode;

fn main() -> ExitCode {
    let output = deepseek_science_cli::run_cli(std::env::args());
    print!("{}", output.stdout);

    if output.exit_code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(output.exit_code as u8)
    }
}
