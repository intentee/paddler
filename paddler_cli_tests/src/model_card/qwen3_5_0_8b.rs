use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn qwen3_5_0_8b() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "Qwen3.5-0.8B-Q4_K_M.gguf".to_owned(),
            repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
