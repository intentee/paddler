use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn open_jev() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "OpenJev-Q4_K_M.gguf".to_owned(),
            repository_name: "OpenJev-GGUF".to_owned(),
            repository_owner: "ggml-org".to_owned(),
            revision: "d2c376d11b6c466ab7bdc9fa458ace41cdf3e776".to_owned(),
        },
    }
}
