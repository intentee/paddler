use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn smolvlm2_256m_mmproj() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "mmproj-SmolVLM2-256M-Video-Instruct-Q8_0.gguf".to_owned(),
            repository_name: "SmolVLM2-256M-Video-Instruct-GGUF".to_owned(),
            repository_owner: "ggml-org".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
