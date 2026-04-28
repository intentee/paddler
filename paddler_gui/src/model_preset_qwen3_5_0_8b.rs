use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;

use crate::model_preset::ModelPreset;

pub fn preset() -> ModelPreset {
    ModelPreset {
        display_name: "Qwen 3.5 0.8B".to_owned(),
        model: HuggingFaceModelReference {
            repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_owned(),
            filename: "Qwen3.5-0.8B-Q4_K_M.gguf".to_owned(),
            revision: "main".to_owned(),
        },
        multimodal_projection: Some(HuggingFaceModelReference {
            repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_owned(),
            filename: "mmproj-F16.gguf".to_owned(),
            revision: "main".to_owned(),
        }),
        inference_parameters: InferenceParameters::default(),
    }
}
