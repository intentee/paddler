use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn smolvlm2_256m_mmproj() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "mmproj-SmolVLM2-256M-Video-Instruct-Q8_0.gguf".to_owned(),
            repo_id: "ggml-org/SmolVLM2-256M-Video-Instruct-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
