use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn qwen2_5_vl_3b() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "Qwen2.5-VL-3B-Instruct-Q4_K_M.gguf".to_owned(),
            repo_id: "ggml-org/Qwen2.5-VL-3B-Instruct-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
