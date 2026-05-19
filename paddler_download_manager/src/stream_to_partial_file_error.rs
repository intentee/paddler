use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StreamToPartialFileError {
    #[error("stream error: {0}")]
    Stream(#[source] reqwest::Error),

    #[error("write error: {0}")]
    Write(#[source] io::Error),
}
