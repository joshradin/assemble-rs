use assemble::execute_v2;

use assemble_core::text_factory::BuildResultString;
use std::process::ExitCode;
use std::time::Instant;

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
