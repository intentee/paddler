use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn t5_small() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "t5-small.bf16.gguf".to_owned(),
            repository_name: "t5-small-GGUF".to_owned(),
            repository_owner: "Xiaojian9992024".to_owned(),
            revision: "24967e4daac0f75eb447dc651d6232f344486f5f".to_owned(),
        },
    }
}
