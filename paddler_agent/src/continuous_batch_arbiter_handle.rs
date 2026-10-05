use std::sync::Arc;
use std::thread;

use anyhow::Context as _;
use anyhow::Result;
use tokio::task::spawn_blocking;

use crate::continuous_batch_request_preparer::ContinuousBatchRequestPreparer;
use crate::join_scheduler_thread::join_scheduler_thread;

pub struct ContinuousBatchArbiterHandle {
    pub request_preparer: Arc<ContinuousBatchRequestPreparer>,
    pub scheduler_thread_handle: thread::JoinHandle<Result<()>>,
}

impl ContinuousBatchArbiterHandle {
    pub async fn shutdown(self) -> Result<()> {
        let Self {
            request_preparer,
            scheduler_thread_handle,
        } = self;

        request_preparer
            .shut_down_scheduler_after_pending_preparations()
            .await;

        spawn_blocking(move || {
            drop(request_preparer);

            join_scheduler_thread(scheduler_thread_handle)
        })
        .await
        .context("Failed to join the scheduler shutdown task")?
    }
}
