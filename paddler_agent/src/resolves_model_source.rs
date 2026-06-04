use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::slot_aggregated_status::SlotAggregatedStatus;

#[async_trait]
pub trait ResolvesModelSource {
    async fn resolve(
        &self,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution>;
}
