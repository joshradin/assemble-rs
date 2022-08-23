use std::error::Error;
use assemble::execute;
use std::fmt::Display;
use std::process::{exit, ExitCode};

fn main() -> ExitCode {
    match execute() {
        Ok(_) => {
            ExitCode::SUCCESS
        }
        Err(_) => {
            ExitCode::FAILURE
        }
    }
}
