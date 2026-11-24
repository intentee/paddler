use anyhow::Result;
use async_trait::async_trait;

use crate::process_end::ProcessEnd;

#[async_trait]
pub trait ManagedProcess: Send {
    async fn exited(&mut self) -> ProcessEnd;

    /// # Errors
    ///
    /// Fails when the process cannot be asked to terminate.
    fn terminate(&mut self) -> Result<()>;
}

impl dyn ManagedProcess {
    /// # Errors
    ///
    /// Fails when the process cannot be asked to terminate, and returns its failure when it does not end cleanly.
    pub async fn shutdown(mut self: Box<Self>) -> Result<()> {
        self.terminate()?;
        self.exited().await.into_result()
    }
}
