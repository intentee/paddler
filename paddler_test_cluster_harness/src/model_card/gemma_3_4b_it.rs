use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn gemma_3_4b_it() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "gemma-3-4b-it-Q4_K_M.gguf".to_owned(),
            repo_id: "unsloth/gemma-3-4b-it-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
