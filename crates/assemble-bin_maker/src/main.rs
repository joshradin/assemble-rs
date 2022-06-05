use std::ffi::OsString;
use std::fs::File;
use std::path::PathBuf;
use assemble_core::task::Action;
use assemble_core::Project;
use assemble_std::tasks::Empty;

use clap::Parser;
use log::info;
use simple_logger::SimpleLogger;

use assemble_bin_maker::yaml::AssembleYamlConfig;

#[derive(Debug, clap::Parser)]
struct BinMakerArgs {
    /// The file to get tasks for.
    #[clap(short = 'f', long = "file", default_value = "assemble.yaml")]
    assemble_file: PathBuf,
}

fn main() {
    SimpleLogger::new().init().unwrap();

    let args: BinMakerArgs = BinMakerArgs::parse();
    let file = File::open(&args.assemble_file).unwrap();

    info!("Using file: {file:?}");

    let assemble_config: AssembleYamlConfig = serde_yaml::from_reader(file).expect("couldn't parse file");
    info!("config: {:#?}", assemble_config);
}
