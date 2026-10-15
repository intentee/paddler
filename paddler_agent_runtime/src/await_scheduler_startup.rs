use std::thread::JoinHandle;

use tokio::select;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use crate::join_scheduler_thread_in_background::join_scheduler_thread_in_background;
use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_handle::SchedulerHandle;
use crate::scheduler_ready::SchedulerReady;
use crate::scheduler_spawn_outcome::SchedulerSpawnOutcome;

pub async fn await_scheduler_startup<TPreparation, TError>(
    cancellation_token: &CancellationToken,
    mut scheduler_startup_rx: UnboundedReceiver<Result<SchedulerReady<TPreparation>, TError>>,
    scheduler_thread_handle: JoinHandle<()>,
) -> Result<SchedulerSpawnOutcome<TPreparation>, TError>
where
    TPreparation: PreparesSchedulerCommand,
    TError: Send + 'static,
{
    let scheduler_startup = select! {
        biased;
        () = cancellation_token.cancelled() => None,
        scheduler_startup = scheduler_startup_rx.recv() => scheduler_startup,
    };

    match scheduler_startup {
        Some(Ok(SchedulerReady {
            model_metadata,
            request_preparer,
        })) => Ok(SchedulerSpawnOutcome::Ready(SchedulerHandle {
            model_metadata,
            request_preparer,
            scheduler_thread_handle,
        })),
        Some(Err(startup_error)) => {
            join_scheduler_thread_in_background(scheduler_startup_rx, scheduler_thread_handle)
                .await;

            Err(startup_error)
        }
        None => {
            join_scheduler_thread_in_background(scheduler_startup_rx, scheduler_thread_handle)
                .await;

            Ok(SchedulerSpawnOutcome::Cancelled)
        }
    }
}
