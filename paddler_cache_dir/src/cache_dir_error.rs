use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheDirError {
    #[error("{variable} is not set; cannot derive the paddler cache directory")]
    HomeVariableUnset { variable: &'static str },
}
