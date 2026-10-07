use std::sync::Arc;
use std::thread::JoinHandle;

use crate::join_scheduler_thread_in_background::join_scheduler_thread_in_background;
use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_request_preparer::SchedulerRequestPreparer;

pub struct SchedulerHandle<TPreparation: PreparesSchedulerCommand, TError> {
    pub request_preparer: Arc<SchedulerRequestPreparer<TPreparation>>,
    pub scheduler_thread_handle: JoinHandle<Result<(), TError>>,
}

impl<TPreparation, TError> SchedulerHandle<TPreparation, TError>
where
    TPreparation: PreparesSchedulerCommand,
    TError: Send + 'static,
{
    pub async fn shut_down(self) -> Result<(), TError> {
        let Self {
            request_preparer,
            scheduler_thread_handle,
        } = self;

        request_preparer
            .stop_scheduler_after_pending_preparations()
            .await;

        join_scheduler_thread_in_background(request_preparer, scheduler_thread_handle).await
    }
}
