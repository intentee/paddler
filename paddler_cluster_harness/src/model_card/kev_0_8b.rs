use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn kev_0_8b() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "Kev-0.8B-BF16.gguf".to_owned(),
            repository_name: "Kev-0.8B-GGUF".to_owned(),
            repository_owner: "ggml-org".to_owned(),
            revision: "9231cb757e3320120077e42e52371971968f008d".to_owned(),
        },
    }
}
