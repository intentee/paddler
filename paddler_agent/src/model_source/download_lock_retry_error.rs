#[derive(Debug, thiserror::Error)]
pub enum DownloadLockRetryError {
    #[error(
        "Hugging Face model download for '{model_path}' was cancelled while waiting for the download lock '{lock_path}'"
    )]
    Cancelled {
        lock_path: String,
        model_path: String,
    },

    #[error(
        "Failed to acquire download lock '{lock_path}'. Is more than one agent running on this machine?"
    )]
    LockStillUnavailable { lock_path: String },
}
