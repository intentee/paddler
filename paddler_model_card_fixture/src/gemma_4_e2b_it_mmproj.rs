use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn gemma_4_e2b_it_mmproj() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "mmproj-F16.gguf".to_owned(),
            repository_name: "gemma-4-E2B-it-GGUF".to_owned(),
            repository_owner: "unsloth".to_owned(),
            revision: "0314792d7f1f7e229411f620751375812bb9faf2".to_owned(),
        },
    }
}
