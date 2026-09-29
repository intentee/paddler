use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn ministral_3_8b_reasoning() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "Ministral-3-8B-Reasoning-2512-Q4_K_M.gguf".to_owned(),
            repo_id: "unsloth/Ministral-3-8B-Reasoning-2512-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
