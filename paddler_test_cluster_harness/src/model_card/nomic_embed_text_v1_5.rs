use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn nomic_embed_text_v1_5() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "nomic-embed-text-v1.5.Q2_K.gguf".to_owned(),
            repo_id: "nomic-ai/nomic-embed-text-v1.5-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
