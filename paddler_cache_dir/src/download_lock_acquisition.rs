use crate::cached_downloaded_model_lock::CachedDownloadedModelLock;

#[derive(Debug)]
pub enum DownloadLockAcquisition {
    Acquired(CachedDownloadedModelLock),
    HeldByAnotherProcess,
}
