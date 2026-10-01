use std::io;
use std::path::PathBuf;

use hf_hub::api::tokio::ApiError;
use reqwest::Error as ReqwestError;
use thiserror::Error;

use paddler_cache_dir::cache_dir_error::CacheDirError;
use paddler_download_manager::download_error::DownloadError;

#[derive(Debug, Error)]
pub enum ModelSourceError {
    #[error("Unable to resolve the model cache directory")]
    CacheDirectoryUnresolvable(#[source] CacheDirError),
    #[error("Unable to set up the model downloader")]
    DownloaderUnavailable(#[source] ReqwestError),
    #[error("Failed to download '{url}'")]
    DownloadFailed {
        url: String,
        #[source]
        source: DownloadError,
    },
    #[error("Cannot download '{url}': another agent on this host is downloading it")]
    DownloadLockHeldByAnotherProcess { url: String },
    #[error("Unable to set up the Hugging Face client")]
    HuggingFaceClientUnavailable(#[source] ApiError),
    #[error("Failed to download model '{model_path}' from Hugging Face")]
    HuggingFaceDownloadFailed {
        model_path: String,
        #[source]
        source: ApiError,
    },
    #[error(
        "Failed to acquire download lock '{lock_path}'. Is more than one agent running on this machine?"
    )]
    HuggingFaceDownloadLockUnavailable { lock_path: String },
    #[error(
        "Model '{model_path}' does not exist on Hugging Face. Not attempting to download it again."
    )]
    HuggingFaceModelKnownToBeMissing { model_path: String },
    #[error("Model '{model_path}' does not exist on Hugging Face.")]
    HuggingFaceModelMissing { model_path: String },
    #[error("You do not have enough permissions to download '{model_path}' from Hugging Face.")]
    HuggingFacePermissionDenied { model_path: String },
    #[error("Local file does not exist: {path_display}", path_display = path.display())]
    LocalFileMissing { path: PathBuf },
    #[error("Unable to check whether the local model file '{path_display}' exists", path_display = path.display())]
    LocalFileUncheckable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
