use fslock::LockFile;

#[derive(Debug)]
pub struct CachedDownloadedModelLock {
    _lock_file: LockFile,
}

impl CachedDownloadedModelLock {
    #[must_use]
    pub const fn new(lock_file: LockFile) -> Self {
        Self {
            _lock_file: lock_file,
        }
    }
}
