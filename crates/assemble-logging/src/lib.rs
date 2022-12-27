//! Controls the logging used with assemble

use log::{Log, Metadata, Record};

/// The logging control struct
#[derive(Debug)]
pub struct Logging;

impl Logging {}

/// The logging intercept, per thread
#[derive(Debug)]
pub struct AssembleLog {
    header: String,
    out_lock: (),
}

impl AssembleLog {}

impl Log for AssembleLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        todo!()
    }

    fn log(&self, record: &Record) {
        todo!()
    }

    fn flush(&self) {
        todo!()
    }
}
