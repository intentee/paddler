use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait Handler {
    async fn handle(&self, shutdown: CancellationToken) -> Result<()>;
}
