use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("URL '{url}' is malformed: {source}")]
    InvalidUrl {
        url: String,
        #[source]
        source: url::ParseError,
    },

    #[error("URL '{url}' returned 404 Not Found")]
    NotFound { url: String },

    #[error("URL '{url}' returned {status}")]
    PermissionDenied {
        url: String,
        status: reqwest::StatusCode,
    },

    #[error("URL '{url}' returned 416 Range Not Satisfiable; '{partial_path_display}' was discarded", partial_path_display = partial_path.display())]
    PartialFileStale { url: String, partial_path: PathBuf },

    #[error("URL '{url}' failed after {attempts} attempts: {source}")]
    NetworkExhausted {
        url: String,
        attempts: u32,
        #[source]
        source: anyhow::Error,
    },

    #[error("I/O on '{path_display}': {source}", path_display = path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
