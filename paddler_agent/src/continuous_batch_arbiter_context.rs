use std::sync::Arc;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

use crate::model_metadata_holder::ModelMetadataHolder;

#[derive(Clone)]
pub struct ContinuousBatchArbiterContext {
    pub agent_name: Option<String>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}
