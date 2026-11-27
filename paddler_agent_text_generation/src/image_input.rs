use llama_cpp_bindings_types::input_media_capabilities::InputMediaCapabilities;

use crate::conversation_media::ConversationMedia;
use crate::prompt_modality::PromptModality;
use crate::text_generation_error::TextGenerationError;

pub enum ImageInput {
    Supported,
    Unsupported,
}

impl From<InputMediaCapabilities> for ImageInput {
    fn from(InputMediaCapabilities { image, .. }: InputMediaCapabilities) -> Self {
        if image {
            Self::Supported
        } else {
            Self::Unsupported
        }
    }
}

impl ImageInput {
    /// # Errors
    ///
    /// Returns [`TextGenerationError::MultimodalNotSupported`] when the prompt carries images the model cannot see.
    pub fn prompt_modality_for(
        &self,
        media: &[ConversationMedia],
    ) -> Result<PromptModality, TextGenerationError> {
        if media.is_empty() {
            return Ok(PromptModality::TextOnly);
        }

        match self {
            Self::Unsupported if media.iter().any(ConversationMedia::is_image) => {
                Err(TextGenerationError::MultimodalNotSupported)
            }
            Self::Supported | Self::Unsupported => Ok(PromptModality::Multimodal),
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::audio_format::AudioFormat;
    use paddler_messaging::image_url::ImageUrl;
    use paddler_messaging::input_audio::InputAudio;

    use super::ImageInput;
    use crate::conversation_media::ConversationMedia;
    use crate::prompt_modality::PromptModality;
    use crate::text_generation_error::TextGenerationError;

    #[test]
    fn rejects_images_the_model_cannot_see() {
        assert!(matches!(
            ImageInput::Unsupported.prompt_modality_for(&[ConversationMedia::Image(ImageUrl {
                url: "data:image/png;base64,AAAA".to_owned(),
            })]),
            Err(TextGenerationError::MultimodalNotSupported)
        ));
    }

    #[test]
    fn audio_alone_needs_no_image_input() {
        assert!(matches!(
            ImageInput::Unsupported.prompt_modality_for(&[ConversationMedia::Audio(InputAudio {
                data: "UklGRg==".to_owned(),
                format: AudioFormat::Wav,
            })]),
            Ok(PromptModality::Multimodal)
        ));
    }
}
