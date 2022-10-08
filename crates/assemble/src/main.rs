use assemble::execute;

use std::process::ExitCode;

fn main() -> ExitCode {
    match execute() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
