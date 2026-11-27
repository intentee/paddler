use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn laya() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "Laya-Q8_0.gguf".to_owned(),
            repository_name: "Laya-GGUF".to_owned(),
            repository_owner: "ggml-org".to_owned(),
            revision: "8d62fa9af5c4d00435ee8ceddff7a2d822c8bce5".to_owned(),
        },
    }
}
