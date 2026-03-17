use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::oneshot;

#[async_trait]
pub trait Handler {
    async fn handle(&self, shutdown_rx: oneshot::Receiver<()>) -> Result<()>;
}
