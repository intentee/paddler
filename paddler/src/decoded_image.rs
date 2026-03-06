use std::io::Cursor;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use image::ImageFormat;
use image::imageops::FilterType;
use paddler_types::image_url::ImageUrl;

use crate::decoded_image_error::DecodedImageError;

#[derive(Debug)]
pub struct DecodedImage {
    pub data: Vec<u8>,
}

impl DecodedImage {
    pub fn from_data_uri(image_url: &ImageUrl) -> Result<Self, DecodedImageError> {
        let url = &image_url.url;

        let after_data = url
            .strip_prefix("data:")
            .ok_or(DecodedImageError::RemoteUrlNotSupported)?;

        let (_metadata, encoded_data) = after_data
            .split_once(',')
            .ok_or(DecodedImageError::MissingCommaSeparator)?;

        let data = BASE64_STANDARD.decode(encoded_data).map_err(|err| {
            DecodedImageError::InvalidBase64Payload {
                message: err.to_string(),
            }
        })?;

        Ok(Self { data })
    }

    pub fn resized_to_fit(self, max_dimension: u32) -> Result<Self, DecodedImageError> {
        if max_dimension == 0 {
            return Ok(self);
        }

        let dynamic_image =
            image::load_from_memory(&self.data).map_err(|err| DecodedImageError::ResizeFailed {
                message: err.to_string(),
            })?;

        let width = dynamic_image.width();
        let height = dynamic_image.height();

        if width <= max_dimension && height <= max_dimension {
            return Ok(self);
        }

        let resized = dynamic_image.resize(max_dimension, max_dimension, FilterType::Lanczos3);

        let mut output_buffer = Cursor::new(Vec::new());

        resized
            .write_to(&mut output_buffer, ImageFormat::Jpeg)
            .map_err(|err| DecodedImageError::ResizeFailed {
                message: err.to_string(),
            })?;

        Ok(Self {
            data: output_buffer.into_inner(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_jpeg(width: u32, height: u32) -> Vec<u8> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Jpeg)
            .expect("Failed to encode test JPEG");

        output_buffer.into_inner()
    }

    #[test]
    fn test_decodes_valid_png_data_uri() {
        let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47];
        let encoded = BASE64_STANDARD.encode(&png_bytes);
        let image_url = ImageUrl {
            url: format!("data:image/png;base64,{encoded}"),
        };

        let result = DecodedImage::from_data_uri(&image_url).unwrap();

        assert_eq!(result.data, png_bytes);
    }

    #[test]
    fn test_rejects_remote_url() {
        let image_url = ImageUrl {
            url: "https://example.com/image.png".to_string(),
        };

        let result = DecodedImage::from_data_uri(&image_url);

        assert!(matches!(
            result,
            Err(DecodedImageError::RemoteUrlNotSupported)
        ));
    }

    #[test]
    fn test_rejects_data_uri_without_comma() {
        let image_url = ImageUrl {
            url: "data:image/png;base64".to_string(),
        };

        let result = DecodedImage::from_data_uri(&image_url);

        assert!(matches!(
            result,
            Err(DecodedImageError::MissingCommaSeparator)
        ));
    }

    #[test]
    fn test_rejects_invalid_base64_payload() {
        let image_url = ImageUrl {
            url: "data:image/png;base64,!!!not-valid-base64!!!".to_string(),
        };

        let result = DecodedImage::from_data_uri(&image_url);

        assert!(matches!(
            result,
            Err(DecodedImageError::InvalidBase64Payload { .. })
        ));
    }

    #[test]
    fn test_resized_to_fit_shrinks_oversized_image() {
        let original_data = create_test_jpeg(2000, 1500);
        let decoded_image = DecodedImage {
            data: original_data,
        };

        let resized = decoded_image.resized_to_fit(1024).unwrap();

        let result_image = image::load_from_memory(&resized.data).unwrap();

        assert!(result_image.width() <= 1024);
        assert!(result_image.height() <= 1024);
    }

    #[test]
    fn test_resized_to_fit_preserves_aspect_ratio() {
        let original_data = create_test_jpeg(2000, 1000);
        let decoded_image = DecodedImage {
            data: original_data,
        };

        let resized = decoded_image.resized_to_fit(1000).unwrap();

        let result_image = image::load_from_memory(&resized.data).unwrap();

        assert_eq!(result_image.width(), 1000);
        assert_eq!(result_image.height(), 500);
    }

    #[test]
    fn test_resized_to_fit_skips_small_image() {
        let original_data = create_test_jpeg(512, 256);
        let original_len = original_data.len();
        let decoded_image = DecodedImage {
            data: original_data,
        };

        let resized = decoded_image.resized_to_fit(1024).unwrap();

        assert_eq!(resized.data.len(), original_len);
    }

    #[test]
    fn test_resized_to_fit_disabled_when_zero() {
        let original_data = create_test_jpeg(2000, 1500);
        let original_len = original_data.len();
        let decoded_image = DecodedImage {
            data: original_data,
        };

        let resized = decoded_image.resized_to_fit(0).unwrap();

        assert_eq!(resized.data.len(), original_len);
    }

    #[test]
    fn test_resized_to_fit_with_llamas_fixture() {
        let fixture_data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../paddler_model_tests/fixtures/llamas.jpg"
        ))
        .expect("Failed to read llamas.jpg fixture");

        let original_image = image::load_from_memory(&fixture_data).unwrap();

        assert_eq!(original_image.width(), 640);
        assert_eq!(original_image.height(), 427);

        let decoded_image = DecodedImage { data: fixture_data };
        let resized = decoded_image.resized_to_fit(320).unwrap();

        let result_image = image::load_from_memory(&resized.data).unwrap();

        assert_eq!(result_image.width(), 320);
        assert_eq!(result_image.height(), 214);
    }
}
