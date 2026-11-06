use llama_cpp_bindings::chat_prompt_part::ChatPromptPart;
use llama_cpp_bindings::mtmd::mtmd_bitmap::MtmdBitmap;
use llama_cpp_bindings::mtmd::mtmd_input_part::MtmdInputPart;
use llama_cpp_bindings::mtmd::mtmd_input_text_part::MtmdInputTextPart;

use paddler_messaging::media_marker::MediaMarker;

use crate::text_generation_error::TextGenerationError;

fn text_part<'bitmap>(text_segment: &str, parse_special: bool) -> MtmdInputPart<'bitmap> {
    MtmdInputPart::Text(MtmdInputTextPart {
        parse_special,
        text: text_segment.to_owned(),
    })
}

pub struct MediaMarkedPrompt<'prompt> {
    pub media_marker: &'prompt MediaMarker,
    pub prompt_parts: &'prompt [ChatPromptPart],
}

impl MediaMarkedPrompt<'_> {
    pub fn interleave_with<'bitmap>(
        &self,
        bitmaps: &'bitmap [MtmdBitmap],
    ) -> Result<Vec<MtmdInputPart<'bitmap>>, TextGenerationError> {
        let marker = self.media_marker.marker.as_str();
        let rendered_media_markers: usize = self
            .prompt_parts
            .iter()
            .map(|prompt_part| prompt_part.text().matches(marker).count())
            .sum();

        if rendered_media_markers != bitmaps.len() {
            return Err(TextGenerationError::RenderedMediaMarkersMismatchImages {
                images: bitmaps.len(),
                rendered_media_markers,
            });
        }

        let mut remaining_bitmaps = bitmaps.iter().map(MtmdInputPart::Bitmap);
        let mut parts = Vec::with_capacity(self.prompt_parts.len() + 2 * bitmaps.len());

        for prompt_part in self.prompt_parts {
            let parse_special = matches!(prompt_part, ChatPromptPart::Template(_));

            for (segment_index, text_segment) in prompt_part.text().split(marker).enumerate() {
                if segment_index > 0 {
                    parts.extend(remaining_bitmaps.next());
                }

                parts.push(text_part(text_segment, parse_special));
            }
        }

        Ok(parts)
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::chat_prompt_part::ChatPromptPart;
    use llama_cpp_bindings::mtmd::mtmd_bitmap::MtmdBitmap;
    use llama_cpp_bindings::mtmd::mtmd_input_part::MtmdInputPart;

    use paddler_messaging::media_marker::MediaMarker;

    use super::MediaMarkedPrompt;
    use crate::text_generation_error::TextGenerationError;

    const MEDIA_MARKER: &str = "<__media_test__>";

    fn image() -> MtmdBitmap {
        MtmdBitmap::from_image_data(2, 2, &[128; 2 * 2 * 3]).unwrap()
    }

    fn part_shape(part: &MtmdInputPart<'_>) -> String {
        match part {
            MtmdInputPart::Bitmap(_) => "bitmap".to_owned(),
            MtmdInputPart::Text(text_part) => {
                format!("{:?} special={}", text_part.text, text_part.parse_special)
            }
        }
    }

    #[test]
    fn places_each_image_where_its_marker_was_rendered_inside_user_input() {
        let media_marker = MediaMarker::new(MEDIA_MARKER.to_owned());
        let images = [image(), image()];
        let prompt_parts = [
            ChatPromptPart::Template("<|user|>".to_owned()),
            ChatPromptPart::Input(format!("Compare {MEDIA_MARKER} with {MEDIA_MARKER}")),
            ChatPromptPart::Template("<|end|>".to_owned()),
        ];

        assert_eq!(
            MediaMarkedPrompt {
                media_marker: &media_marker,
                prompt_parts: &prompt_parts,
            }
            .interleave_with(&images)
            .unwrap()
            .iter()
            .map(part_shape)
            .collect::<Vec<String>>(),
            [
                "\"<|user|>\" special=true",
                "\"Compare \" special=false",
                "bitmap",
                "\" with \" special=false",
                "bitmap",
                "\"\" special=false",
                "\"<|end|>\" special=true",
            ]
        );
    }

    #[test]
    fn a_rendering_that_lost_an_image_marker_is_rejected() {
        let media_marker = MediaMarker::new(MEDIA_MARKER.to_owned());

        assert_eq!(
            MediaMarkedPrompt {
                media_marker: &media_marker,
                prompt_parts: &[ChatPromptPart::Input("Describe nothing".to_owned())],
            }
            .interleave_with(&[image()])
            .err()
            .as_ref()
            .map(ToString::to_string),
            Some(
                TextGenerationError::RenderedMediaMarkersMismatchImages {
                    images: 1,
                    rendered_media_markers: 0,
                }
                .to_string()
            )
        );
    }
}
