use std::thread::spawn;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use crate::await_scheduler_startup::await_scheduler_startup;
use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_ready::SchedulerReady;
use crate::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use crate::send_startup_signal::send_startup_signal;

pub async fn start_scheduler_thread<TPreparation, TError, TStartScheduler>(
    cancellation_token: &CancellationToken,
    start_scheduler: TStartScheduler,
) -> Result<SchedulerSpawnOutcome<TPreparation>, TError>
where
    TPreparation: PreparesSchedulerCommand,
    TError: Send + 'static,
    TStartScheduler: FnOnce(
            &CancellationToken,
            &UnboundedSender<Result<SchedulerReady<TPreparation>, TError>>,
        ) -> Result<(), TError>
        + Send
        + 'static,
{
    let (scheduler_startup_tx, scheduler_startup_rx) = unbounded_channel();
    let agent_shutdown = cancellation_token.clone();
    let scheduler_thread_handle = spawn(move || {
        if let Err(startup_error) = start_scheduler(&agent_shutdown, &scheduler_startup_tx) {
            send_startup_signal(&scheduler_startup_tx, Err(startup_error));
        }
    });

    await_scheduler_startup(
        cancellation_token,
        scheduler_startup_rx,
        scheduler_thread_handle,
    )
    .await
}
