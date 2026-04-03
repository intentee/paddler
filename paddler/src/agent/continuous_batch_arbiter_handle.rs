use std::sync::mpsc::Sender;
use std::thread;

use anyhow::Result;
use anyhow::anyhow;

use crate::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;

pub struct ContinuousBatchArbiterHandle {
    pub command_tx: Sender<ContinuousBatchSchedulerCommand>,
    pub scheduler_thread_handle: Option<thread::JoinHandle<Result<()>>>,
}

impl ContinuousBatchArbiterHandle {
    pub fn shutdown(&mut self) -> Result<()> {
        self.command_tx
            .send(ContinuousBatchSchedulerCommand::Shutdown)
            .map_err(|err| anyhow!("Failed to send shutdown command: {err}"))?;

        let thread_handle = self
            .scheduler_thread_handle
            .take()
            .ok_or_else(|| anyhow!("Scheduler thread handle already consumed"))?;

        thread_handle
            .join()
            .map_err(|err| anyhow!("Failed to join scheduler thread: {err:?}"))??;

        Ok(())
    }
}

impl Drop for ContinuousBatchArbiterHandle {
    fn drop(&mut self) {
        let _ = self
            .command_tx
            .send(ContinuousBatchSchedulerCommand::Shutdown);
    }
}
