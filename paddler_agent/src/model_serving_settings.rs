use std::path::PathBuf;

use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::model_placement::ModelPlacement;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;

#[derive(Clone, Debug, PartialEq)]
pub struct ModelServingSettings {
    pub model_capabilities: ModelCapabilitiesSettings,
    pub model_path: PathBuf,
    pub model_runtime_parameters: DesiredModelRuntimeParameters,
    pub text_generation: TextGenerationSettings,
}

impl ModelServingSettings {
    #[must_use]
    pub fn llama_model_settings(&self) -> LlamaModelSettings {
        LlamaModelSettings {
            model_path: self.model_path.clone(),
            placement: ModelPlacement::from(&self.model_runtime_parameters),
        }
    }
}
