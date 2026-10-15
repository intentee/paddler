use std::sync::Arc;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

use crate::model_metadata_holder::ModelMetadataHolder;
use crate::reports_available_parallelism::ReportsAvailableParallelism;
use crate::shared_llama_backend::SharedLlamaBackend;

#[derive(Clone)]
pub struct InferenceRuntimeContext {
    pub agent_name: Option<String>,
    pub available_parallelism: Arc<dyn ReportsAvailableParallelism>,
    pub llama_backend: Arc<SharedLlamaBackend>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}
