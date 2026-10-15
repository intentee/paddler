use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn qwen2_5_vl_3b_mmproj() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "mmproj-Qwen2.5-VL-3B-Instruct-Q8_0.gguf".to_owned(),
            repo_id: "ggml-org/Qwen2.5-VL-3B-Instruct-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
