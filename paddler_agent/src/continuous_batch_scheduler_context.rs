use std::path::PathBuf;
use std::sync::Arc;

use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::mtmd::MtmdContext;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::media_marker::MediaMarker;

use crate::chat_template_renderer::ChatTemplateRenderer;

pub struct ContinuousBatchSchedulerContext {
    pub agent_name: Option<String>,
    pub chat_template_renderer: Option<Arc<ChatTemplateRenderer>>,
    pub desired_slots_total: i32,
    pub inference_parameters: InferenceParameters,
    pub media_marker: MediaMarker,
    pub model: Arc<LlamaModel>,
    pub model_path: PathBuf,
    pub multimodal_context: Option<Arc<MtmdContext>>,
    pub sequence_context_size: u32,
    pub token_bos_str: String,
    pub token_eos_str: String,
    pub token_nl_str: String,
}
