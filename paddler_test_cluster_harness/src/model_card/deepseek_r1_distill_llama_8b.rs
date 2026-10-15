use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn deepseek_r1_distill_llama_8b() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf".to_owned(),
            repo_id: "unsloth/DeepSeek-R1-Distill-Llama-8B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
