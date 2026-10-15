use std::sync::Arc;
use std::thread::JoinHandle;

use paddler_messaging::model_metadata::ModelMetadata;

use crate::join_scheduler_thread_in_background::join_scheduler_thread_in_background;
use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_request_preparer::SchedulerRequestPreparer;

pub struct SchedulerHandle<TPreparation: PreparesSchedulerCommand> {
    pub model_metadata: ModelMetadata,
    pub request_preparer: Arc<SchedulerRequestPreparer<TPreparation>>,
    pub scheduler_thread_handle: JoinHandle<()>,
}

impl<TPreparation> SchedulerHandle<TPreparation>
where
    TPreparation: PreparesSchedulerCommand,
{
    pub async fn shut_down(self) {
        let Self {
            request_preparer,
            scheduler_thread_handle,
            ..
        } = self;

        request_preparer
            .stop_scheduler_after_pending_preparations()
            .await;

        join_scheduler_thread_in_background(request_preparer, scheduler_thread_handle).await;
    }
}
