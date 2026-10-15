use std::sync::Arc;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
use paddler_agent_runtime::shared_llama_backend::SharedLlamaBackend;
use paddler_agent_runtime::system_parallelism::SystemParallelism;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

pub fn inference_runtime_context_fixture() -> InferenceRuntimeContext {
    InferenceRuntimeContext {
        agent_name: Some("agent".to_owned()),
        available_parallelism: Arc::new(SystemParallelism),
        llama_backend: Arc::new(SharedLlamaBackend::default()),
        model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
        slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
    }
}
