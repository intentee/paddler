use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheDirError {
    #[error("HOME is not set; cannot derive the paddler cache directory")]
    HomeVariableUnset,
}
