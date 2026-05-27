use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn qwen3_embedding_0_6b() -> ModelCard {
    ModelCard {
        gpu_layer_count: 28,
        reference: HuggingFaceModelReference {
            filename: "Qwen3-Embedding-0.6B-Q8_0.gguf".to_owned(),
            repo_id: "Qwen/Qwen3-Embedding-0.6B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
