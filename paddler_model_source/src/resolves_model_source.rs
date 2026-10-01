use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::model_source_error::ModelSourceError;

#[async_trait]
pub trait ResolvesModelSource {
    async fn resolve(
        &self,
        cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError>;
}
