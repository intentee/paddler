use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::slot_aggregated_status::SlotAggregatedStatus;

#[async_trait]
pub trait ResolvesModelSource {
    async fn resolve(
        &self,
        cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution>;
}
