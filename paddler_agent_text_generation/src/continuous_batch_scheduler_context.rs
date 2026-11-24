use llama_cpp_model::llama_model::LlamaModel;

use crate::multimodal_ingestion::MultimodalIngestion;

pub struct ContinuousBatchSchedulerContext<'model> {
    pub agent_name: Option<String>,
    pub desired_slots_total: u16,
    pub model: &'model LlamaModel,
    pub multimodal_ingestion: MultimodalIngestion,
    pub sequence_context_size: u32,
}
