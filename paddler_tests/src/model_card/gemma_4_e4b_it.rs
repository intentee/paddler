use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn gemma_4_e4b_it() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "gemma-4-E4B-it-Q4_K_M.gguf".to_owned(),
            repo_id: "unsloth/gemma-4-E4B-it-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
