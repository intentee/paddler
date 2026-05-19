use std::io;

use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadAttemptError {
    #[error("client error: {0}")]
    ClientError(StatusCode),

    #[error("io")]
    Io(#[from] io::Error),

    #[error("not found")]
    NotFound,

    #[error("partial file stale")]
    PartialFileStale,

    #[error("permission denied: {0}")]
    PermissionDenied(StatusCode),

    #[error("server returned error status: {0}")]
    ServerError(StatusCode),

    #[error("download interrupted: {0}")]
    Interrupted(anyhow::Error),

    #[error("server unreachable: {0}")]
    Unreachable(anyhow::Error),
}
