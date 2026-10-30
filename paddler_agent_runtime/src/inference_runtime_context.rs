use std::sync::Arc;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

use crate::agent_vram_residencies::AgentVramResidencies;
use crate::inference_runtime_context_params::InferenceRuntimeContextParams;
use crate::model_metadata_holder::ModelMetadataHolder;
use crate::reports_available_parallelism::ReportsAvailableParallelism;
use crate::shared_llama_backend::SharedLlamaBackend;
use crate::system_parallelism::SystemParallelism;

#[derive(Clone)]
pub struct InferenceRuntimeContext {
    pub agent_name: Option<String>,
    pub available_parallelism: Arc<dyn ReportsAvailableParallelism>,
    pub llama_backend: Arc<SharedLlamaBackend>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    pub vram_residencies: AgentVramResidencies,
}

impl InferenceRuntimeContext {
    #[must_use]
    pub fn new(
        InferenceRuntimeContextParams { agent_name, slots }: InferenceRuntimeContextParams,
    ) -> Self {
        Self {
            agent_name,
            available_parallelism: Arc::new(SystemParallelism),
            llama_backend: Arc::new(SharedLlamaBackend::default()),
            model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(slots)),
            vram_residencies: AgentVramResidencies::default(),
        }
    }
}
