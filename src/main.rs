mod cli;
mod glob;
mod json;
mod package;
mod runner;
mod shell;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run_from_env() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            if !error.silent {
                eprintln!("ERROR: {}", error.message);
            }
            ExitCode::from(error.exit_code)
        }
    }
}
