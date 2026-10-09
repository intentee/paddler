use anyhow::Result;
use async_trait::async_trait;

use crate::process_end::ProcessEnd;

#[async_trait]
pub trait ManagedProcess: Send {
    fn terminate(&mut self) -> Result<()>;

    async fn exited(&mut self) -> ProcessEnd;

    async fn shutdown(mut self: Box<Self>) -> Result<()> {
        self.terminate()?;

        match self.exited().await {
            ProcessEnd::Clean => Ok(()),
            ProcessEnd::Failed(exit_failure) => Err(exit_failure),
        }
    }
}
