use anyhow::Error as AnyhowError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceThreadError {
    #[error("A Paddler service stopped with an error")]
    ServiceRunFailed {
        #[source]
        source: AnyhowError,
    },
    #[error("A Paddler service thread panicked")]
    ServiceThreadPanicked,
}
