use std::io;
use std::path::PathBuf;
use std::str::Utf8Error;

use hf_hub::HFError;
use reqwest::Error as ReqwestError;
use thiserror::Error;
use url::ParseError;

use paddler_cache_dir::cache_dir_error::CacheDirError;
use paddler_download_manager::download_error::DownloadError;

#[derive(Debug, Error)]
pub enum ModelSourceError {
    #[error(
        "Agent URI '{uri}' does not name an absolute local path: use agent:///absolute/path/to/model.gguf"
    )]
    AgentUriNotAnAbsoluteLocalPath { uri: String },
    #[error("Agent URI '{uri}' is not canonical: use '{canonical_uri}'")]
    AgentUriNotCanonical { canonical_uri: String, uri: String },
    #[error("Unable to resolve the model cache directory: {0}")]
    CacheDirectoryUnresolvable(#[source] CacheDirError),
    #[error("Unable to set up the model download client: {0}")]
    DownloadClientUnavailable(#[source] ReqwestError),
    #[error("Failed to download '{url}': {source}")]
    DownloadFailed {
        url: String,
        #[source]
        source: DownloadError,
    },
    #[error("Cannot download '{url}': another agent on this host is downloading it")]
    DownloadLockHeldByAnotherProcess { url: String },
    #[error("Unable to look up model '{model_path}' in the Hugging Face cache: {source}")]
    HuggingFaceCacheLookupFailed {
        model_path: String,
        #[source]
        source: HFError,
    },
    #[error("Unable to set up the Hugging Face client: {0}")]
    HuggingFaceClientUnavailable(#[source] HFError),
    #[error("Failed to download model '{model_path}' from Hugging Face: {source}")]
    HuggingFaceDownloadFailed {
        model_path: String,
        #[source]
        source: HFError,
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
    #[error(
        "Hugging Face model URI '{uri}' must point at a file: https://huggingface.co/<owner>/<repo>/blob/<revision>/<file>"
    )]
    HuggingFaceModelUriMalformed { uri: String },
    #[error(
        "Hugging Face model URI '{uri}' has a path component that is not valid UTF-8 once percent-decoded: {source}"
    )]
    HuggingFaceModelUriComponentNotUtf8 {
        uri: String,
        #[source]
        source: Utf8Error,
    },
    #[error("You do not have enough permissions to download '{model_path}' from Hugging Face.")]
    HuggingFacePermissionDenied { model_path: String },
    #[error("Local file does not exist: {path_display}", path_display = path.display())]
    LocalFileMissing { path: PathBuf },
    #[error("Unable to check whether the local model file '{path_display}' exists: {source}", path_display = path.display())]
    LocalFileUncheckable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Local model path '{path_display}' climbs to a parent directory", path_display = path.display())]
    LocalModelPathClimbsToParentDirectory { path: PathBuf },
    #[error("Local model path '{path_display}' is not absolute", path_display = path.display())]
    LocalModelPathNotAbsolute { path: PathBuf },
    #[error(
        "Model URI '{uri}' is neither a Hugging Face, an agent:// nor a downloadable URI: {source}"
    )]
    ModelUriNotDownloadable {
        uri: String,
        #[source]
        source: DownloadError,
    },
    #[error("Model URI '{uri}' cannot be parsed: {source}")]
    ModelUriUnparseable {
        uri: String,
        #[source]
        source: ParseError,
    },
}
