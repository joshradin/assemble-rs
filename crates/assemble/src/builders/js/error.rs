use crate::builders::js::JavascriptBuilder;
use crate::error::AssembleError;
use assemble_js::javascript::FileError;
use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum JavascriptError {
    #[error("No settings file could be found")]
    MissingSettingsFile,
    #[error(transparent)]
    RQuickJsError(#[from] rquickjs::Error),
    #[error("{1:#?}: {0}")]
    RQuickJsErrorWithFile(rquickjs::Error, PathBuf),
    #[error(transparent)]
    FileError(#[from] assemble_js::javascript::FileError),
    #[error(transparent)]
    IoError(#[from] io::Error),
}
