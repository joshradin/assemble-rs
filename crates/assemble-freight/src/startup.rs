//! Connects freight using the startup api

use crate::FreightArgs;
use assemble_core::prelude::StartParameter;

impl From<FreightArgs> for StartParameter {
    fn from(args: FreightArgs) -> Self {
        let mut start_parameter = StartParameter::new();

        start_parameter
            .task_requests_mut()
            .extend(args.task_requests_raw().into_iter().map(String::clone));

        start_parameter.set_backtrace(args.backtrace());

        start_parameter.set_logging(args.logging().clone());
        start_parameter.set_mode(args.logging().console);
        start_parameter.properties_mut().extend(args.properties().properties());

        start_parameter.set_workers(args.workers());

        start_parameter
    }
}

