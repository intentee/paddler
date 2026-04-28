use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;

use crate::model_preset::ModelPreset;

pub fn preset() -> ModelPreset {
    ModelPreset {
        display_name: "Qwen 3 0.6B".to_owned(),
        model: HuggingFaceModelReference {
            repo_id: "unsloth/Qwen3-0.6B-GGUF".to_owned(),
            filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
            revision: "main".to_owned(),
        },
        multimodal_projection: None,
        inference_parameters: InferenceParameters::default(),
    }
}
