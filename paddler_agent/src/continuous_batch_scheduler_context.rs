use std::path::PathBuf;
use std::sync::Arc;

use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::mtmd::MtmdContext;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::chat_template_renderer::ChatTemplateRenderer;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub struct ContinuousBatchSchedulerContext {
    pub agent_name: Option<String>,
    pub chat_template_renderer: Option<Arc<ChatTemplateRenderer>>,
    pub desired_slots_total: i32,
    pub inference_parameters: InferenceParameters,
    pub model: Arc<LlamaModel>,
    pub model_path: PathBuf,
    pub multimodal_context: Option<Arc<MtmdContext>>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    pub token_bos_str: String,
    pub token_eos_str: String,
    pub token_nl_str: String,
}
