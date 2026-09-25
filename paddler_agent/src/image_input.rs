use llama_cpp_bindings::mtmd::MtmdBitmap;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::multimodal_prompt_support::MultimodalPromptSupport;
use crate::prepared_multimodal_prompt::PreparedMultimodalPrompt;

pub enum ImageInput {
    Supported(MultimodalPromptSupport),
    Unsupported,
}

impl ImageInput {
    pub fn prepare_multimodal_prompt(
        &self,
        bitmaps: Vec<MtmdBitmap>,
        text: String,
    ) -> Result<PreparedMultimodalPrompt, GenerationRequestRejection> {
        match self {
            Self::Supported(MultimodalPromptSupport {
                multimodal_context,
                n_batch,
            }) => Ok(PreparedMultimodalPrompt {
                bitmaps,
                multimodal_context: multimodal_context.clone(),
                n_batch: *n_batch,
                text,
            }),
            Self::Unsupported => Err(GenerationRequestRejection::MultimodalNotSupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use super::ImageInput;
    use crate::generation_request_rejection::GenerationRequestRejection;

    #[test]
    fn rejects_images_when_no_multimodal_projection_is_loaded() {
        assert_eq!(
            ImageInput::Unsupported
                .prepare_multimodal_prompt(Vec::new(), "Describe".to_owned())
                .err()
                .map(|rejection| discriminant(&rejection)),
            Some(discriminant(
                &GenerationRequestRejection::MultimodalNotSupported
            ))
        );
    }
}
