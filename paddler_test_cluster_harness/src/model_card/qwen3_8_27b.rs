use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn qwen3_8_27b() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "Qwen3.8-27B-UD-IQ1_S.gguf".to_owned(),
            repo_id: "unsloth/Qwen3.8-27B-GGUF".to_owned(),
            revision: "4ca720788d1e01f1bff70c033e0d0028fd02e502".to_owned(),
        },
    }
}
