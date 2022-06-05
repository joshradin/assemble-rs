use log::LevelFilter;

/// Provides helpful logging args for clap clis
#[derive(Debug, clap::Args)]
pub struct LoggingArgs {
    /// Only display error level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["warn", "info", "debug", "trace"]))]
    error: bool,
    /// Display warning and above level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["error", "info", "debug", "trace"]))]
    warn: bool,
    /// Display info and above level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["error", "warn", "debug", "trace"]))]
    info: bool,
    /// Display debug and above level log messages
    #[clap(long)]
    #[clap(conflicts_with_all(&["error", "warn", "info", "trace"]))]
    debug: bool,
    /// Display trace and above level log messages
    #[clap(long)]
    #[clap(conflicts_with_all(&["error", "warn", "info", "debug"]))]
    trace: bool,
}

impl LoggingArgs {

    /// Get the level filter from this args
    pub fn log_level_filter(&self) -> LevelFilter {
        if self.error {
            LevelFilter::Error
        } else if self.warn {
            LevelFilter::Warn
        } else if self.info {
            LevelFilter::Info
        } else if self.debug {
            LevelFilter::Debug
        } else if self.trace {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        }
    }
}