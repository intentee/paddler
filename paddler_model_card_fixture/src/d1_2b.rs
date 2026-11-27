use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn d1_2b() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "d1-2B-Q4_0.gguf".to_owned(),
            repository_name: "d1-2B-GGUF".to_owned(),
            repository_owner: "LiquidAI".to_owned(),
            revision: "4a181094d1b6addfaa3a0956f78e7ab2aa406a0c".to_owned(),
        },
    }
}
