use std::io;
use std::num::NonZeroUsize;

pub trait ReportsAvailableParallelism: Send + Sync {
    fn available_parallelism(&self) -> io::Result<NonZeroUsize>;
}
