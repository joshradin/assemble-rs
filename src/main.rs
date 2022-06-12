use assemble_core::logging::LoggingArgs;
use clap::Parser;

#[derive(Debug, Parser)]
struct AssembleArgs {
    /// The tasks to run
    tasks: Vec<String>,
    /// Force to remake the assemble-daemon binary for this project
    #[clap(long)]
    reload: bool,
    #[clap(flatten)]
    logging: LoggingArgs,
}

fn main() {
    let args = AssembleArgs::parse();
    println!("args: {:#?}", args);
}
