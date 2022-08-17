use assemble::execute;
use std::fmt::Display;
use std::process::exit;

fn main() {
    match execute() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            exit(101);
        }
    }
}
