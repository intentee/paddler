use anyhow::Result;
use async_trait::async_trait;

use crate::process_end::ProcessEnd;

#[async_trait]
pub trait ManagedProcess: Send {
    fn terminate(&mut self) -> Result<()>;

    async fn exited(&mut self) -> ProcessEnd;
}

impl dyn ManagedProcess {
    pub async fn shutdown(mut self: Box<Self>) -> Result<()> {
        self.terminate()?;
        self.exited().await.into_result()
    }
}
