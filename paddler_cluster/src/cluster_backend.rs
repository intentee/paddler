use anyhow::Result;
use async_trait::async_trait;

use crate::provisioned_backend::ProvisionedBackend;

#[async_trait]
pub trait ClusterBackend {
    async fn provision(&self) -> Result<ProvisionedBackend>;
}
