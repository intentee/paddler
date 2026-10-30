use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::error::TryRecvError;

use crate::scheduler_command::SchedulerCommand;
use crate::scheduler_message::SchedulerMessage;

pub struct SchedulerInbox<TCommand: SchedulerCommand> {
    pub scheduler_message_rx: UnboundedReceiver<SchedulerMessage<TCommand>>,
    pub shutdown_requested: bool,
}

impl<TCommand: SchedulerCommand> SchedulerInbox<TCommand> {
    #[must_use]
    pub const fn new(scheduler_message_rx: UnboundedReceiver<SchedulerMessage<TCommand>>) -> Self {
        Self {
            scheduler_message_rx,
            shutdown_requested: false,
        }
    }

    pub fn take_next_waiting_command(&mut self) -> Option<TCommand> {
        match self.scheduler_message_rx.try_recv() {
            Ok(SchedulerMessage::Command(command)) => Some(command),
            Ok(SchedulerMessage::Shutdown) | Err(TryRecvError::Disconnected) => {
                self.shutdown_requested = true;

                None
            }
            Err(TryRecvError::Empty) => None,
        }
    }

    pub fn wait_for_next_command(&mut self) -> Option<TCommand> {
        match self.scheduler_message_rx.blocking_recv() {
            Some(SchedulerMessage::Command(command)) => Some(command),
            Some(SchedulerMessage::Shutdown) | None => {
                self.shutdown_requested = true;

                None
            }
        }
    }

    pub fn reject_waiting_commands(&mut self, agent_name: Option<&str>) {
        self.scheduler_message_rx.close();

        while let Ok(scheduler_message) = self.scheduler_message_rx.try_recv() {
            match scheduler_message {
                SchedulerMessage::Command(command) => {
                    command.reject_because_the_scheduler_stopped(agent_name);
                }
                SchedulerMessage::Shutdown => {}
            }
        }
    }
}
