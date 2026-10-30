use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn ultravox_v0_5_llama_3_2_1b() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "Llama-3.2-1B-Instruct-Q4_K_M.gguf".to_owned(),
            repository_name: "ultravox-v0_5-llama-3_2-1b-GGUF".to_owned(),
            repository_owner: "ggml-org".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
