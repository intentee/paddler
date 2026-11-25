use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn tiny_granite_switch_4_1_8b_for_testing() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "tiny-granite-switch-4.1-8b-for-testing-Q4_0.gguf".to_owned(),
            repository_name: "tiny-granite-switch-4.1-8b-for-testing-GGUF".to_owned(),
            repository_owner: "ggml-org-dev".to_owned(),
            revision: "9ef521a9b09bdf67afb2fce15673f518de304d87".to_owned(),
        },
    }
}
