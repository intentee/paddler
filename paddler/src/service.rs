use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;

#[async_trait]
pub trait Service: Send + 'static {
    fn name(&self) -> &'static str;

    async fn run(&mut self, shutdown_rx: broadcast::Receiver<()>) -> Result<()>;
}
