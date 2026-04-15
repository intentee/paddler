use std::sync::mpsc::SendError;
use std::sync::mpsc::Sender;
use std::thread;

use anyhow::Result;
use anyhow::anyhow;

use crate::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;

pub struct ContinuousBatchArbiterHandle {
    pub command_tx: Sender<ContinuousBatchSchedulerCommand>,
    pub scheduler_thread_handle: thread::JoinHandle<Result<()>>,
}

impl ContinuousBatchArbiterHandle {
    pub fn shutdown(self) -> Result<()> {
        if let Err(SendError(_unsent_command)) = self
            .command_tx
            .send(ContinuousBatchSchedulerCommand::Shutdown)
        {
            // Scheduler thread already dropped its receiver; join below is authoritative.
        }

        self.scheduler_thread_handle
            .join()
            .map_err(|err| anyhow!("Failed to join scheduler thread: {err:?}"))?
    }
}
