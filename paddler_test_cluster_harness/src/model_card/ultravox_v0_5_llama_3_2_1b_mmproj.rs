use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn ultravox_v0_5_llama_3_2_1b_mmproj() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "mmproj-ultravox-v0_5-llama-3_2-1b-f16.gguf".to_owned(),
            repository_name: "ultravox-v0_5-llama-3_2-1b-GGUF".to_owned(),
            repository_owner: "ggml-org".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
