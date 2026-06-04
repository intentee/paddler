use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ManagedProcess: Send {
    async fn shutdown(&mut self) -> Result<()>;
}
