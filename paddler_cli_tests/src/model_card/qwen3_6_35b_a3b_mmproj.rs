use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn qwen3_6_35b_a3b_mmproj() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "mmproj-F16.gguf".to_owned(),
            repo_id: "unsloth/Qwen3.6-35B-A3B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
