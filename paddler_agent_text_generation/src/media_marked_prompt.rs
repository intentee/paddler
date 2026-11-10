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
    /// # Errors
    ///
    /// Returns [`TextGenerationError::RenderedMediaMarkersMismatchImages`] when the rendered prompt holds a different number of media markers than there are images.
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
        let mut parts = Vec::new();

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

    fn part_placement(part: &MtmdInputPart<'_>) -> String {
        match part {
            MtmdInputPart::Bitmap(_) => "bitmap".to_owned(),
            MtmdInputPart::Text(text_part) => format!("{:?}", text_part.text),
        }
    }

    fn inspect_interleaved_comparison_of_two_images<TInspection>(
        inspect: impl FnOnce(&[MtmdInputPart<'_>]) -> TInspection,
    ) -> TInspection {
        let media_marker = MediaMarker::new(MEDIA_MARKER.to_owned());
        let images = [image(), image()];
        let prompt_parts = [
            ChatPromptPart::Template("<|user|>".to_owned()),
            ChatPromptPart::Input(format!("Compare {MEDIA_MARKER} with {MEDIA_MARKER}")),
            ChatPromptPart::Template("<|end|>".to_owned()),
        ];

        inspect(
            &MediaMarkedPrompt {
                media_marker: &media_marker,
                prompt_parts: &prompt_parts,
            }
            .interleave_with(&images)
            .unwrap(),
        )
    }

    #[test]
    fn places_each_image_where_its_marker_was_rendered_inside_user_input() {
        assert_eq!(
            inspect_interleaved_comparison_of_two_images(|parts| parts
                .iter()
                .map(part_placement)
                .collect::<Vec<String>>()),
            [
                "\"<|user|>\"",
                "\"Compare \"",
                "bitmap",
                "\" with \"",
                "bitmap",
                "\"\"",
                "\"<|end|>\"",
            ]
        );
    }

    #[test]
    fn parses_special_tokens_only_in_template_parts() {
        assert_eq!(
            inspect_interleaved_comparison_of_two_images(|parts| parts
                .iter()
                .filter_map(|part| match part {
                    MtmdInputPart::Bitmap(_) => None,
                    MtmdInputPart::Text(text_part) => Some(text_part.parse_special),
                })
                .collect::<Vec<bool>>()),
            [true, false, false, false, true]
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
