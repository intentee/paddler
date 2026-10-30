use std::path::PathBuf;

use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;

use crate::applicable_decisions::ApplicableDecisions;
use crate::inference_capabilities::InferenceCapabilities;

#[derive(Clone, Debug, PartialEq)]
pub struct ModelServingSettings {
    pub capabilities:
        InferenceCapabilities<ApplicableDecisions, PoolingType, TextGenerationSettings>,
    pub model_path: PathBuf,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl ModelServingSettings {
    #[must_use]
    pub fn llama_model_settings(&self) -> LlamaModelSettings {
        LlamaModelSettings {
            model_path: self.model_path.clone(),
            n_gpu_layers: self.model_runtime_parameters.n_gpu_layers,
        }
    }
}
