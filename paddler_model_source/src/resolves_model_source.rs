use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::model_source_error::ModelSourceError;

pub trait ResolvesModelSource {
    fn resolve(
        &self,
        cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> impl Future<Output = Result<DesiredModelResolution, ModelSourceError>> + Send;
}
