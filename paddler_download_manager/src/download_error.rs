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

    #[error("server unreachable for URL '{url}': {source}")]
    DownloadServerIsUnreachable {
        url: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("server returned error status {status} for URL '{url}'")]
    DownloadServerErrored {
        url: String,
        status: reqwest::StatusCode,
    },

    #[error("download interrupted while downloading URL '{url}': {source}")]
    DownloadInterrupted {
        url: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("I/O on '{path_display}': {source}", path_display = path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("cache write denied at '{path_display}': {source}", path_display = path.display())]
    CachePermissionDenied {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("cache disk full at '{path_display}': {source}", path_display = path.display())]
    CacheDiskFull {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
