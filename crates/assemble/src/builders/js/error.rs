#[derive(Debug, thiserror::Error)]
pub enum JavascriptError {
    #[error("No settings file could be found")]
    MissingSettingsFile,
    #[error(transparent)]
    RQuickJsError(#[from] rquickjs::Error)
}