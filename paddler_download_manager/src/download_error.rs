use std::io;
use std::path::PathBuf;

use reqwest::Error as ReqwestError;
use reqwest::StatusCode;
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("URL '{url}' is malformed: {source}")]
    InvalidUrl {
        url: String,
        #[source]
        source: ParseError,
    },

    #[error("URL '{url}' has unsupported scheme '{scheme}'; expected http or https")]
    UnsupportedUrlScheme { url: String, scheme: String },

    #[error("URL '{url}' returned 404 Not Found")]
    NotFound { url: String },

    #[error("URL '{url}' returned {status}")]
    PermissionDenied { url: String, status: StatusCode },

    #[error("URL '{url}' returned 416 Range Not Satisfiable; '{partial_path_display}' was discarded", partial_path_display = partial_path.display())]
    PartialFileStale { url: String, partial_path: PathBuf },

    #[error("'{partial_path_display}' is a directory, not a partial download file", partial_path_display = partial_path.display())]
    PartialPathIsADirectory { partial_path: PathBuf },

    #[error("server unreachable for URL '{url}': {source}")]
    DownloadServerIsUnreachable {
        url: String,
        #[source]
        source: ReqwestError,
    },

    #[error("server returned error status {status} for URL '{url}'")]
    DownloadServerErrored { url: String, status: StatusCode },

    #[error("server rejected request to URL '{url}' with status {status}")]
    DownloadServerRejectedRequest { url: String, status: StatusCode },

    #[error("download interrupted while downloading URL '{url}': {source}")]
    DownloadInterrupted {
        url: String,
        #[source]
        source: ReqwestError,
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

impl DownloadError {
    #[must_use]
    pub fn cache_failure(path: PathBuf, source: io::Error) -> Self {
        match source.kind() {
            io::ErrorKind::PermissionDenied => Self::CachePermissionDenied { path, source },
            io::ErrorKind::StorageFull => Self::CacheDiskFull { path, source },
            _ => Self::Io { path, source },
        }
    }
}
