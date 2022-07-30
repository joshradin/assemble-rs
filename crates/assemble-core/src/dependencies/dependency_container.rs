use std::sync::{Arc, Mutex};
use crate::dependencies::RegistryContainer;

pub struct DependencyContainer {
    registries: Arc<Mutex<RegistryContainer>>

}