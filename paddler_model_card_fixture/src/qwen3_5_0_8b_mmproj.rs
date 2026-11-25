use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn qwen3_5_0_8b_mmproj() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "mmproj-F16.gguf".to_owned(),
            repository_name: "Qwen3.5-0.8B-GGUF".to_owned(),
            repository_owner: "unsloth".to_owned(),
            revision: "6ab461498e2023f6e3c1baea90a8f0fe38ab64d0".to_owned(),
        },
    }
}
