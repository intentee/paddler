use std::sync::Arc;
use std::thread::JoinHandle;

use tokio::sync::oneshot;
use tokio::sync::oneshot::error::RecvError;
use tokio_util::sync::CancellationToken;

use crate::join_scheduler_thread_in_background::join_scheduler_thread_in_background;
use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_handle::SchedulerHandle;
use crate::scheduler_request_preparer::SchedulerRequestPreparer;
use crate::scheduler_spawn_outcome::SchedulerSpawnOutcome;

pub async fn await_scheduler_startup<TPreparation, TError>(
    cancellation_token: &CancellationToken,
    scheduler_ready_rx: oneshot::Receiver<Arc<SchedulerRequestPreparer<TPreparation>>>,
    scheduler_thread_handle: JoinHandle<Result<(), TError>>,
) -> Result<SchedulerSpawnOutcome<TPreparation, TError>, TError>
where
    TPreparation: PreparesSchedulerCommand,
    TError: Send + 'static,
{
    match cancellation_token
        .run_until_cancelled(scheduler_ready_rx)
        .await
    {
        Some(Ok(request_preparer)) => Ok(SchedulerSpawnOutcome::Ready(SchedulerHandle {
            request_preparer,
            scheduler_thread_handle,
        })),
        None | Some(Err(RecvError { .. })) => {
            join_scheduler_thread_in_background((), scheduler_thread_handle)
                .await
                .map(|()| SchedulerSpawnOutcome::Cancelled)
        }
    }
}
