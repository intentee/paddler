use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdInputPart;
use llama_cpp_bindings::mtmd::MtmdInputTextPart;

use paddler_messaging::media_marker::MediaMarker;

use crate::text_generation_error::TextGenerationError;

fn text_part<'bitmap>(text_segment: &str) -> MtmdInputPart<'bitmap> {
    MtmdInputPart::Text(MtmdInputTextPart {
        parse_special: true,
        text: text_segment.to_owned(),
    })
}

pub struct MediaMarkedPrompt<'prompt> {
    pub media_marker: &'prompt MediaMarker,
    pub prompt: &'prompt str,
}

impl MediaMarkedPrompt<'_> {
    pub fn interleave_with<'bitmap>(
        &self,
        bitmaps: &'bitmap [MtmdBitmap],
    ) -> Result<Vec<MtmdInputPart<'bitmap>>, TextGenerationError> {
        let text_segments: Vec<&str> = self
            .prompt
            .split(self.media_marker.marker.as_str())
            .collect();

        match text_segments.split_last() {
            Some((trailing_text_segment, leading_text_segments))
                if leading_text_segments.len() == bitmaps.len() =>
            {
                let mut parts = Vec::with_capacity(text_segments.len() + bitmaps.len());

                for (text_segment, bitmap) in leading_text_segments.iter().zip(bitmaps) {
                    parts.push(text_part(text_segment));
                    parts.push(MtmdInputPart::Bitmap(bitmap));
                }

                parts.push(text_part(trailing_text_segment));

                Ok(parts)
            }
            _ => Err(TextGenerationError::RenderedMediaMarkersMismatchImages {
                images: bitmaps.len(),
                rendered_media_markers: text_segments.len().saturating_sub(1),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::mtmd::MtmdBitmap;
    use llama_cpp_bindings::mtmd::MtmdInputPart;

    use paddler_messaging::media_marker::MediaMarker;

    use super::MediaMarkedPrompt;
    use crate::text_generation_error::TextGenerationError;

    const MEDIA_MARKER: &str = "<__media_test__>";

    fn image() -> MtmdBitmap {
        MtmdBitmap::from_image_data(2, 2, &[128; 2 * 2 * 3]).unwrap()
    }

    fn interleaving_failure(prompt: &str, images: &[MtmdBitmap]) -> Option<String> {
        let media_marker = MediaMarker::new(MEDIA_MARKER.to_owned());

        MediaMarkedPrompt {
            media_marker: &media_marker,
            prompt,
        }
        .interleave_with(images)
        .err()
        .as_ref()
        .map(ToString::to_string)
    }

    #[test]
    fn places_each_image_where_its_marker_was_rendered() {
        let media_marker = MediaMarker::new(MEDIA_MARKER.to_owned());
        let images = [image(), image()];

        assert_eq!(
            MediaMarkedPrompt {
                media_marker: &media_marker,
                prompt: &format!("Compare {MEDIA_MARKER} with {MEDIA_MARKER} please"),
            }
            .interleave_with(&images)
            .unwrap()
            .iter()
            .map(|part| matches!(part, MtmdInputPart::Bitmap(_)))
            .collect::<Vec<_>>(),
            [false, true, false, true, false]
        );
    }

    #[test]
    fn a_rendering_that_lost_an_image_marker_is_rejected() {
        assert_eq!(
            interleaving_failure("Describe nothing", &[image()]),
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
