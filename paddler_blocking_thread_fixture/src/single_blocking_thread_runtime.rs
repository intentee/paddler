use std::io::Result as IoResult;

use tokio::runtime::Builder;
use tokio::runtime::Runtime;

const ONLY_BLOCKING_THREAD: usize = 1;

pub fn single_blocking_thread_runtime() -> IoResult<Runtime> {
    Builder::new_current_thread()
        .max_blocking_threads(ONLY_BLOCKING_THREAD)
        .enable_all()
        .build()
}
