//! contains class declarations in javascript various assemble listeners

use rquickjs::bind;
use assemble_std::__export::ProjectResult;
use assemble_std::prelude::listeners::Listener;
use assemble_std::prelude::{Assemble, Settings};
use assemble_std::startup::listeners::BuildListener;

#[bind]
#[quickjs(object, bare)]
mod build_listener {
    #[derive(Debug)]
    pub struct BuildListener;

    impl BuildListener {
        pub fn new() -> Self { BuildListener }

        pub fn settings_evaluated(&self) {

        }
    }
}

impl Listener for build_listener::BuildListener {
    type Listened =Assemble;

    fn add_listener(self, freight: &mut Self::Listened) -> ProjectResult {
        freight.add_build_listener(self)
    }
}

impl BuildListener for build_listener::BuildListener {
    fn settings_evaluated(&mut self, settings: &Settings) -> ProjectResult {
        todo!()
    }
}