use std::sync::Arc;
use std::sync::mpsc::SendError;
use std::sync::mpsc::Sender;

use log::info;
use tokio_util::task::TaskTracker;

use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_command::SchedulerCommand as _;
use crate::scheduler_message::SchedulerMessage;

pub struct SchedulerRequestPreparer<TPreparation: PreparesSchedulerCommand> {
    pub agent_name: Option<String>,
    pub preparation: TPreparation,
    pub preparation_tasks: TaskTracker,
    pub scheduler_message_tx: Sender<SchedulerMessage<TPreparation::Command>>,
}

impl<TPreparation: PreparesSchedulerCommand> SchedulerRequestPreparer<TPreparation> {
    pub fn prepare(self: &Arc<Self>, request: TPreparation::Request) {
        let request_preparer = self.clone();

        self.preparation_tasks.spawn_blocking(move || {
            if let Some(command) = request_preparer
                .preparation
                .prepare_scheduler_command(request_preparer.agent_name.as_deref(), request)
            {
                request_preparer.send(SchedulerMessage::Command(command));
            }
        });
    }

    pub async fn stop_scheduler_after_pending_preparations(&self) {
        self.preparation_tasks.close();
        self.preparation_tasks.wait().await;
        self.send(SchedulerMessage::Shutdown);
    }

    fn send(&self, scheduler_message: SchedulerMessage<TPreparation::Command>) {
        if let Err(SendError(undelivered_message)) =
            self.scheduler_message_tx.send(scheduler_message)
        {
            match undelivered_message {
                SchedulerMessage::Command(command) => {
                    command.reject_because_the_scheduler_stopped(self.agent_name.as_deref());
                }
                SchedulerMessage::Shutdown => {
                    info!(
                        "{:?}: the scheduler stopped before its shutdown was requested",
                        self.agent_name
                    );
                }
            }
        }
    }
}
