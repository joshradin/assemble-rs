use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, Read, stdin};
use std::path::PathBuf;
use assemble_core::task::Action;
use assemble_core::Project;
use assemble_std::tasks::Empty;

use clap::Parser;
use log::{debug, info, trace};
use simple_logger::SimpleLogger;
use assemble_bin_maker::binary_building::TaskSpec;

use assemble_bin_maker::yaml::AssembleYamlConfig;
use assemble_core::logging::LoggingArgs;

#[derive(Debug, clap::Parser)]
#[clap(author, version, author)]
struct BinMakerArgs {
    /// The file to get tasks from.
    ///
    /// If no file is present, then stdin is used.
    #[clap(short = 'f', long = "file")]
    assemble_file: Option<PathBuf>,
    /// The type of input file.
    ///
    /// Must be specified if stdin is used as input
    #[clap(arg_enum, short = 't', long = "type")]
    #[clap(required_unless_present("assemble-file"))]
    #[clap(default_value_if("assemble-file", None, Some("auto")))]
    assemble_file_type: Option<InputFileType>,
    /// The compilation mechanism that's used to build the executable
    #[clap(arg_enum, short, long)]
    compile: BinIntermediateType,
    #[clap(flatten)]
    logging: LoggingArgs,
}

#[derive(Debug, Clone, clap::ArgEnum)]
pub enum BinIntermediateType {
    /// Use the rust back-end
    #[cfg(feature = "rust")]
    Rust,
}

#[derive(Debug, Clone, clap::ArgEnum)]
pub enum InputFileType {
    /// Automatically determine file from extension
    Auto,
    /// Force the input file to be a yaml file
    Yaml
}

fn main() {
    let args: BinMakerArgs = BinMakerArgs::parse();

    args.logging.init_logger();

    debug!("args: {:#?}", args);

    let file: Box<dyn Read> = match &args.assemble_file {
        None => {
            info!("Using stdin");
            Box::new(stdin())
        }
        Some(file) => {
            info!("Using file: {file:?}");
            Box::new(File::open(file).unwrap())
        }
    };



    let assemble_config: AssembleYamlConfig = serde_yaml::from_reader(file).expect("couldn't parse file");
    trace!("config: {:#?}", assemble_config);

    let tasks = assemble_config.tasks
        .into_iter()
        .map(|task_dec| TaskSpec::from(task_dec))
        .collect::<Vec<_>>();

    debug!("{:#?}", tasks);
}
