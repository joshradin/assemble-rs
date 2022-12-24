use assemble::execute_v2;

use std::process::ExitCode;
use std::time::Instant;
use assemble_core::text_factory::BuildResultString;

fn main() -> ExitCode {
    let start = Instant::now();
    let res = execute_v2();
    let status = BuildResultString::new(res.is_ok(), start.elapsed());
    println!();
    println!("{}", status);
    match res {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
