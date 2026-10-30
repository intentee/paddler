use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn lfm2_colbert_350m() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "LFM2-ColBERT-350M-Q4_K_M.gguf".to_owned(),
            repository_name: "LFM2-ColBERT-350M-GGUF".to_owned(),
            repository_owner: "LiquidAI".to_owned(),
            revision: "main".to_owned(),
        },
    }
}
