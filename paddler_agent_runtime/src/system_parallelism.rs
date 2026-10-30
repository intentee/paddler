use std::io;
use std::num::NonZeroUsize;
use std::thread::available_parallelism;

use crate::reports_available_parallelism::ReportsAvailableParallelism;

pub struct SystemParallelism;

impl ReportsAvailableParallelism for SystemParallelism {
    fn available_parallelism(&self) -> io::Result<NonZeroUsize> {
        available_parallelism()
    }
}
