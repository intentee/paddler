use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn lfm2_colbert_350m() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "LFM2-ColBERT-350M-Q4_K_M.gguf".to_owned(),
            repository_name: "LFM2-ColBERT-350M-GGUF".to_owned(),
            repository_owner: "LiquidAI".to_owned(),
            revision: "fb69df0f183f45d48f8775bd4170e9f0522c9f6e".to_owned(),
        },
    }
}
