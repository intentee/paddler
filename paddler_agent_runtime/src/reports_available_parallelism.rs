use std::io;
use std::num::NonZeroUsize;

pub trait ReportsAvailableParallelism: Send + Sync {
    /// # Errors
    ///
    /// Returns an [`io::Error`] when the machine does not report how many threads it runs in parallel.
    fn available_parallelism(&self) -> io::Result<NonZeroUsize>;
}
