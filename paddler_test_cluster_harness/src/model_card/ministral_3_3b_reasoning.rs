use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn ministral_3_3b_reasoning() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "Ministral-3-3B-Reasoning-2512-Q4_K_M.gguf".to_owned(),
            repo_id: "unsloth/Ministral-3-3B-Reasoning-2512-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
