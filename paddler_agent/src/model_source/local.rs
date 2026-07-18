use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::fs::try_exists;
use tokio_util::sync::CancellationToken;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::resolves_model_source::ResolvesModelSource;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub struct LocalModelPath {
    pub path: String,
}

impl LocalModelPath {
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self { path }
    }
}

#[async_trait]
impl ResolvesModelSource for LocalModelPath {
    async fn resolve(
        &self,
        _slot_aggregated_status: Arc<SlotAggregatedStatus>,
        _cancellation_token: &CancellationToken,
    ) -> Result<DesiredModelResolution> {
        let local_path = PathBuf::from(&self.path);

        if try_exists(&local_path).await? {
            Ok(DesiredModelResolution::Resolved(local_path))
        } else {
            Ok(DesiredModelResolution::LocalFileMissing(local_path))
        }
    }
}
