use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;

use crate::model_card::ModelCard;

#[must_use]
pub fn minicpm_ocr_jev() -> ModelCard {
    ModelCard {
        reference: HuggingFaceModelReference {
            filename: "MiniCPM-OCR-Jev-IQ3_XS.gguf".to_owned(),
            repository_name: "MiniCPM-OCR-Jev-gguf".to_owned(),
            repository_owner: "openbmb".to_owned(),
            revision: "6bd2b4b932bbab92de97cea4cc6a754db96fc848".to_owned(),
        },
    }
}
