use std::thread;

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::SendError;

use crate::continuous_batch_arbiter_command::ContinuousBatchArbiterCommand;
use crate::join_scheduler_thread::join_scheduler_thread;

pub struct ContinuousBatchArbiterHandle {
    pub command_tx: UnboundedSender<ContinuousBatchArbiterCommand>,
    pub scheduler_thread_handle: thread::JoinHandle<Result<()>>,
}

impl ContinuousBatchArbiterHandle {
    pub fn shutdown(self) -> Result<()> {
        if let Err(SendError(_unsent_command)) = self
            .command_tx
            .send(ContinuousBatchArbiterCommand::Shutdown)
        {
            // Scheduler thread already dropped its receiver; join below is authoritative.
        }

        join_scheduler_thread(self.scheduler_thread_handle)
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::thread;

    use tokio::sync::mpsc::unbounded_channel;

    use super::ContinuousBatchArbiterHandle;
    use crate::continuous_batch_arbiter_command::ContinuousBatchArbiterCommand;

    #[test]
    fn shutdown_sends_command_and_joins_successful_thread() {
        let (command_tx, mut command_rx) = unbounded_channel::<ContinuousBatchArbiterCommand>();
        let scheduler_thread_handle = thread::spawn(move || {
            let received_command = command_rx.blocking_recv().unwrap();

            assert_eq!(
                discriminant(&received_command),
                discriminant(&ContinuousBatchArbiterCommand::Shutdown)
            );

            Ok(())
        });
        let handle = ContinuousBatchArbiterHandle {
            command_tx,
            scheduler_thread_handle,
        };

        handle.shutdown().unwrap();
    }

    #[test]
    fn shutdown_tolerates_dropped_receiver_and_joins() {
        let (command_tx, command_rx) = unbounded_channel::<ContinuousBatchArbiterCommand>();
        let scheduler_thread_handle = thread::spawn(|| Ok(()));

        drop(command_rx);

        let handle = ContinuousBatchArbiterHandle {
            command_tx,
            scheduler_thread_handle,
        };

        handle.shutdown().unwrap();
    }

    #[test]
    fn shutdown_reports_error_when_thread_panics() {
        let (command_tx, mut command_rx) = unbounded_channel::<ContinuousBatchArbiterCommand>();
        let scheduler_thread_handle = thread::spawn(move || {
            drop(command_rx.blocking_recv());

            panic!("scheduler thread crashed");
        });
        let handle = ContinuousBatchArbiterHandle {
            command_tx,
            scheduler_thread_handle,
        };

        let shutdown_error = handle.shutdown().err().unwrap();

        assert_eq!(
            shutdown_error.to_string(),
            "Failed to join scheduler thread: Any { .. }"
        );
    }
}
