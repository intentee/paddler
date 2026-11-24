use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn ministral_3_3b_reasoning() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "Ministral-3-3B-Reasoning-2512-Q4_K_M.gguf".to_owned(),
            repository_name: "Ministral-3-3B-Reasoning-2512-GGUF".to_owned(),
            repository_owner: "unsloth".to_owned(),
            revision: "e22649036c52d4ea95a91a9900dd97b2c431852a".to_owned(),
        },
    }
}
