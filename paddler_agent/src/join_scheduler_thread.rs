use std::thread::JoinHandle;

use anyhow::Result;
use anyhow::anyhow;

pub fn join_scheduler_thread(scheduler_thread_handle: JoinHandle<Result<()>>) -> Result<()> {
    scheduler_thread_handle
        .join()
        .map_err(|err| anyhow!("Failed to join scheduler thread: {err:?}"))?
}
