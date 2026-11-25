use paddler_messaging::image_url::ImageUrl;

use crate::prompt_modality::PromptModality;
use crate::text_generation_error::TextGenerationError;

pub enum ImageInput {
    Supported,
    Unsupported,
}

impl ImageInput {
    /// # Errors
    ///
    /// Returns [`TextGenerationError::MultimodalNotSupported`] when the prompt carries images and no multimodal projection is loaded.
    pub const fn prompt_modality_for(
        &self,
        image_urls: &[ImageUrl],
    ) -> Result<PromptModality, TextGenerationError> {
        if image_urls.is_empty() {
            return Ok(PromptModality::TextOnly);
        }

        match self {
            Self::Supported => Ok(PromptModality::Multimodal),
            Self::Unsupported => Err(TextGenerationError::MultimodalNotSupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::image_url::ImageUrl;

    use super::ImageInput;
    use crate::text_generation_error::TextGenerationError;

    #[test]
    fn rejects_images_when_no_multimodal_projection_is_loaded() {
        assert!(matches!(
            ImageInput::Unsupported.prompt_modality_for(&[ImageUrl {
                url: "data:image/png;base64,AAAA".to_owned(),
            }]),
            Err(TextGenerationError::MultimodalNotSupported)
        ));
    }
}
