use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn ministral_3_8b_reasoning_mmproj() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "mmproj-F16.gguf".to_owned(),
            repository_name: "Ministral-3-8B-Reasoning-2512-GGUF".to_owned(),
            repository_owner: "unsloth".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
