use std::sync::mpsc::Sender;
use std::thread;

use anyhow::Result;
use anyhow::anyhow;

use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;

pub struct ContinuousBatchArbiterHandle {
    pub command_tx: Sender<ContinuousBatchSchedulerCommand>,
    pub scheduler_thread_handle: thread::JoinHandle<Result<()>>,
}

impl ContinuousBatchArbiterHandle {
    pub fn shutdown(self) -> Result<()> {
        let _ = self
            .command_tx
            .send(ContinuousBatchSchedulerCommand::Shutdown);

        self.scheduler_thread_handle
            .join()
            .map_err(|err| anyhow!("Failed to join scheduler thread: {err:?}"))?
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::sync::mpsc::channel;
    use std::thread;

    use super::ContinuousBatchArbiterHandle;
    use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;

    #[test]
    fn shutdown_sends_command_and_joins_successful_thread() {
        let (command_tx, command_rx) = channel::<ContinuousBatchSchedulerCommand>();
        let scheduler_thread_handle = thread::spawn(move || {
            let received_command = command_rx.recv().unwrap();

            assert_eq!(
                discriminant(&received_command),
                discriminant(&ContinuousBatchSchedulerCommand::Shutdown)
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
        let (command_tx, command_rx) = channel::<ContinuousBatchSchedulerCommand>();
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
        let (command_tx, command_rx) = channel::<ContinuousBatchSchedulerCommand>();
        let scheduler_thread_handle = thread::spawn(move || {
            drop(command_rx.recv());

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
