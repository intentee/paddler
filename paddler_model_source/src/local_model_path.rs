use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs::try_exists;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::model_source_error::ModelSourceError;
use crate::resolves_model_source::ResolvesModelSource;

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
        _cancellation_token: &CancellationToken,
        _slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError> {
        let local_path = PathBuf::from(&self.path);

        match try_exists(&local_path).await {
            Ok(true) => Ok(DesiredModelResolution::Resolved(local_path)),
            Ok(false) => Ok(DesiredModelResolution::LocalFileMissing(local_path)),
            Err(source) => Err(ModelSourceError::LocalFileUncheckable {
                path: local_path,
                source,
            }),
        }
    }
}
