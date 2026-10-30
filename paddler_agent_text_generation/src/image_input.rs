use paddler_messaging::chat_template_content_form::ChatTemplateContentForm;
use paddler_messaging::image_url::ImageUrl;

use crate::multimodal_prompt_support::MultimodalPromptSupport;
use crate::prompt_modality::PromptModality;
use crate::text_generation_error::TextGenerationError;

pub enum ImageInput {
    Supported(MultimodalPromptSupport),
    Unsupported,
}

impl ImageInput {
    #[must_use]
    pub const fn chat_template_content_form(&self) -> ChatTemplateContentForm {
        match self {
            Self::Supported(_) => ChatTemplateContentForm::TypedParts,
            Self::Unsupported => ChatTemplateContentForm::JoinedText,
        }
    }

    pub const fn prompt_modality_for(
        &self,
        image_urls: &[ImageUrl],
    ) -> Result<PromptModality<'_>, TextGenerationError> {
        if image_urls.is_empty() {
            return Ok(PromptModality::TextOnly);
        }

        match self {
            Self::Supported(multimodal_prompt_support) => {
                Ok(PromptModality::Multimodal(multimodal_prompt_support))
            }
            Self::Unsupported => Err(TextGenerationError::MultimodalNotSupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use paddler_messaging::image_url::ImageUrl;

    use super::ImageInput;
    use crate::text_generation_error::TextGenerationError;

    #[test]
    fn rejects_images_when_no_multimodal_projection_is_loaded() {
        assert_eq!(
            ImageInput::Unsupported
                .prompt_modality_for(&[ImageUrl {
                    url: "data:image/png;base64,AAAA".to_owned(),
                }])
                .err()
                .map(|rejection| discriminant(&rejection)),
            Some(discriminant(&TextGenerationError::MultimodalNotSupported))
        );
    }
}
