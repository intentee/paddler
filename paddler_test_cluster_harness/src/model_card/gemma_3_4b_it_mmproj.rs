use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn gemma_3_4b_it_mmproj() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "mmproj-F16.gguf".to_owned(),
            repo_id: "unsloth/gemma-3-4b-it-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
