use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn ministral_3_8b_reasoning_mmproj() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "mmproj-F16.gguf".to_owned(),
            repo_id: "unsloth/Ministral-3-8B-Reasoning-2512-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
