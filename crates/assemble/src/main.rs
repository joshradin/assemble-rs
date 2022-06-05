use std::path::PathBuf;
use clap::Parser;
use assemble_core::logging::LoggingArgs;

#[derive(Debug, clap::Parser)]
struct AssembleArgs {
    /// The tasks to run
    tasks: Vec<String>,
    /// Force to remake the assemble binary for this project
    #[clap(long)]
    reload: bool,
    #[clap(flatten)]
    logging: LoggingArgs
}

fn main() {
    let args = AssembleArgs::parse();
    println!("args: {:#?}", args);
}
