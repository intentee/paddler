use std::path::PathBuf;
use std::sync::Arc;

use llama_cpp_2::model::LlamaModel;
use paddler_types::inference_parameters::InferenceParameters;

use crate::chat_template_renderer::ChatTemplateRenderer;

pub struct LlamaCppSlotContext {
    pub agent_name: Option<String>,
    pub chat_template_renderer: Arc<ChatTemplateRenderer>,
    pub inference_parameters: InferenceParameters,
    pub model: Arc<LlamaModel>,
    pub model_path: PathBuf,
    pub token_bos_str: String,
    pub token_eos_str: String,
    pub token_nl_str: String,
}
