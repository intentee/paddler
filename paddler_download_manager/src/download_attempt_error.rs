use std::io;

use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadAttemptError {
    #[error("io")]
    Io(#[from] io::Error),

    #[error("not found")]
    NotFound,

    #[error("partial file stale")]
    PartialFileStale,

    #[error("permission denied: {0}")]
    PermissionDenied(StatusCode),

    #[error("transient: {0}")]
    Transient(anyhow::Error),
}
