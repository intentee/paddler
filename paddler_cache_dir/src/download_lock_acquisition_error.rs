use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadLockAcquisitionError {
    #[error("another agent on this host is currently downloading this URL")]
    AnotherProcessIsDownloading,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl DownloadLockAcquisitionError {
    #[must_use]
    pub const fn is_another_process_downloading(&self) -> bool {
        matches!(self, Self::AnotherProcessIsDownloading)
    }

    #[must_use]
    pub const fn is_io(&self) -> bool {
        matches!(self, Self::Io(_))
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use crate::download_lock_acquisition_error::DownloadLockAcquisitionError;

    #[test]
    fn is_another_process_downloading_returns_true_only_for_that_variant() {
        let another_process = DownloadLockAcquisitionError::AnotherProcessIsDownloading;
        let io_error = DownloadLockAcquisitionError::Io(io::Error::from(io::ErrorKind::NotFound));

        assert!(another_process.is_another_process_downloading());
        assert!(!io_error.is_another_process_downloading());
    }

    #[test]
    fn is_io_returns_true_only_for_io_variant() {
        let io_error = DownloadLockAcquisitionError::Io(io::Error::from(io::ErrorKind::NotFound));
        let another_process = DownloadLockAcquisitionError::AnotherProcessIsDownloading;

        assert!(io_error.is_io());
        assert!(!another_process.is_io());
    }
}
