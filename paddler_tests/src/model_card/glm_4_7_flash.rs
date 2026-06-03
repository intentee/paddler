use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn glm_4_7_flash() -> ModelCard {
    ModelCard {
        gpu_layer_count: 999,
        reference: HuggingFaceModelReference {
            filename: "GLM-4.7-Flash-Q4_K_M.gguf".to_owned(),
            repo_id: "unsloth/GLM-4.7-Flash-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
