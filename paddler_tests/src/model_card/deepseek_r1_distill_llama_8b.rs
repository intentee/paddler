use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn deepseek_r1_distill_llama_8b() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf".to_owned(),
            repo_id: "unsloth/DeepSeek-R1-Distill-Llama-8B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
