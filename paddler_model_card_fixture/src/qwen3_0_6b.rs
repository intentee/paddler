use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn qwen3_0_6b() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
            repository_name: "Qwen3-0.6B-GGUF".to_owned(),
            repository_owner: "Qwen".to_owned(),
            revision: "23749fefcc72300e3a2ad315e1317431b06b590a".to_owned(),
        },
    }
}
