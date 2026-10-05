use paddler_messaging::image_url::ImageUrl;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::multimodal_prompt_support::MultimodalPromptSupport;
use crate::prompt_modality::PromptModality;

pub enum ImageInput {
    Supported(MultimodalPromptSupport),
    Unsupported,
}

impl ImageInput {
    pub const fn prompt_modality_for(
        &self,
        image_urls: &[ImageUrl],
    ) -> Result<PromptModality<'_>, GenerationRequestRejection> {
        if image_urls.is_empty() {
            return Ok(PromptModality::TextOnly);
        }

        match self {
            Self::Supported(multimodal_prompt_support) => {
                Ok(PromptModality::Multimodal(multimodal_prompt_support))
            }
            Self::Unsupported => Err(GenerationRequestRejection::MultimodalNotSupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use paddler_messaging::image_url::ImageUrl;

    use super::ImageInput;
    use crate::generation_request_rejection::GenerationRequestRejection;

    #[test]
    fn rejects_images_when_no_multimodal_projection_is_loaded() {
        assert_eq!(
            ImageInput::Unsupported
                .prompt_modality_for(&[ImageUrl {
                    url: "data:image/png;base64,AAAA".to_owned(),
                }])
                .err()
                .map(|rejection| discriminant(&rejection)),
            Some(discriminant(
                &GenerationRequestRejection::MultimodalNotSupported
            ))
        );
    }
}
