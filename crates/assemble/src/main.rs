use assemble::execute_v2;

use std::process::ExitCode;

fn main() -> ExitCode {
    match execute_v2() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
