use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn tiny_decision_2_0_kai_0_6b_for_testing() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "tiny-Decision-2.0-Kai-0.6B-for-testing-Q4_0.gguf".to_owned(),
            repository_name: "tiny-Decision-2.0-Kai-0.6B-for-testing-GGUF".to_owned(),
            repository_owner: "ggml-org-dev".to_owned(),
            revision: "0f629c57c040fdfccc9d94ec86def6b3e472127f".to_owned(),
        },
    }
}
