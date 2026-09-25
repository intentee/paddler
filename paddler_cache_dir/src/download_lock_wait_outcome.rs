#[derive(Debug, Eq, PartialEq)]
pub enum DownloadLockWaitOutcome {
    Cancelled,
    LockStillUnavailable { lock_path: String },
}
