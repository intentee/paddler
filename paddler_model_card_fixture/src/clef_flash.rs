use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn clef_flash() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "Clef-Flash-Q4_K_M.gguf".to_owned(),
            repository_name: "Clef-Flash-GGUF".to_owned(),
            repository_owner: "ggml-org".to_owned(),
            revision: "c3d925454595d792d5333da8b35f3c09911f7568".to_owned(),
        },
    }
}
