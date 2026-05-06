use std::io::Cursor;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use image::DynamicImage;
use image::ImageFormat;
use image::imageops::FilterType;
use log::info;
use paddler_types::image_url::ImageUrl;
use resvg::tiny_skia::Pixmap;
use resvg::usvg::Options;
use resvg::usvg::Tree as SvgTree;

use crate::decoded_image_error::DecodedImageError;

fn is_svg(data: &[u8]) -> bool {
    let trimmed = match std::str::from_utf8(data) {
        Ok(text) => text.trim_start(),
        Err(_) => return false,
    };

    trimmed.starts_with("<svg") || trimmed.starts_with("<?xml")
}

fn compute_target_dimension(svg_dim: f64, scale: f64) -> Result<u32, DecodedImageError> {
    let target = (svg_dim * scale).ceil();

    if !target.is_finite() || target < 1.0 || target > f64::from(u32::MAX) {
        return Err(DecodedImageError::ConversionFailed {
            message: format!("SVG dimension {target} is out of valid range"),
        });
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounds-checked above: target is in 0..=u32::MAX"
    )]
    Ok(target as u32)
}

fn rasterize_svg_to_dynamic_image(
    data: &[u8],
    max_dimension: u32,
) -> Result<DynamicImage, DecodedImageError> {
    let svg_tree = SvgTree::from_data(data, &Options::default()).map_err(|err| {
        DecodedImageError::ConversionFailed {
            message: format!("Failed to parse SVG: {err}"),
        }
    })?;

    let svg_size = svg_tree.size();
    let svg_width = f64::from(svg_size.width());
    let svg_height = f64::from(svg_size.height());
    let max_dim = f64::from(max_dimension);

    let scale = (max_dim / svg_width).min(max_dim / svg_height).min(1.0);

    let target_width = compute_target_dimension(svg_width, scale)?;
    let target_height = compute_target_dimension(svg_height, scale)?;

    let mut pixmap = Pixmap::new(target_width, target_height).ok_or_else(|| {
        DecodedImageError::ConversionFailed {
            message: "Failed to create pixmap for SVG rasterization".to_owned(),
        }
    })?;

    let render_scale_x = f64::from(target_width) / svg_width;
    let render_scale_y = f64::from(target_height) / svg_height;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Transform::from_scale requires f32; scale factors are small ratios"
    )]
    let transform =
        resvg::tiny_skia::Transform::from_scale(render_scale_x as f32, render_scale_y as f32);

    resvg::render(&svg_tree, transform, &mut pixmap.as_mut());

    let rgba = image::RgbaImage::from_raw(target_width, target_height, pixmap.data().to_vec())
        .ok_or_else(|| DecodedImageError::ConversionFailed {
            message: "rasterized SVG buffer did not match target dimensions".to_owned(),
        })?;

    Ok(DynamicImage::ImageRgba8(rgba))
}

enum LoadedImageOrigin {
    PassThroughEligible,
    NeedsReencode,
}

fn load_supported_image(
    data: &[u8],
    max_dimension: u32,
) -> Result<(DynamicImage, LoadedImageOrigin), DecodedImageError> {
    if is_svg(data) {
        info!("Rasterizing SVG (max_dimension: {max_dimension})");
        let image = rasterize_svg_to_dynamic_image(data, max_dimension)?;
        return Ok((image, LoadedImageOrigin::NeedsReencode));
    }

    let format = image::guess_format(data).map_err(|err| DecodedImageError::ConversionFailed {
        message: err.to_string(),
    })?;

    let origin = match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::Bmp => {
            LoadedImageOrigin::PassThroughEligible
        }
        unsupported if !unsupported.reading_enabled() => {
            return Err(DecodedImageError::UnsupportedFormat {
                format: format!("{unsupported:?}"),
            });
        }
        convertible_format => {
            info!("Converting {convertible_format:?} image to PNG for llama.cpp compatibility");
            LoadedImageOrigin::NeedsReencode
        }
    };

    let image =
        image::load_from_memory(data).map_err(|err| DecodedImageError::ConversionFailed {
            message: err.to_string(),
        })?;

    Ok((image, origin))
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, DecodedImageError> {
    let mut output_buffer = Cursor::new(Vec::new());

    image
        .write_to(&mut output_buffer, ImageFormat::Png)
        .map_err(|err| DecodedImageError::ConversionFailed {
            message: err.to_string(),
        })?;

    Ok(output_buffer.into_inner())
}

fn encode_jpeg(image: &DynamicImage) -> Result<Vec<u8>, DecodedImageError> {
    let mut output_buffer = Cursor::new(Vec::new());

    image
        .write_to(&mut output_buffer, ImageFormat::Jpeg)
        .map_err(|err| DecodedImageError::ResizeFailed {
            message: err.to_string(),
        })?;

    Ok(output_buffer.into_inner())
}

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

    pub fn prepared_for_inference(self, max_dimension: u32) -> Result<Self, DecodedImageError> {
        if max_dimension == 0 {
            return Err(DecodedImageError::InvalidMaxDimension);
        }

        let (image, origin) = load_supported_image(&self.data, max_dimension)?;

        let width = image.width();
        let height = image.height();
        let needs_resize = width > max_dimension || height > max_dimension;

        if needs_resize {
            let resized = image.resize(max_dimension, max_dimension, FilterType::Lanczos3);
            return Ok(Self {
                data: encode_jpeg(&resized)?,
            });
        }

        match origin {
            LoadedImageOrigin::PassThroughEligible => Ok(self),
            LoadedImageOrigin::NeedsReencode => Ok(Self {
                data: encode_png(&image)?,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use anyhow::Result;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use image::ImageFormat;
    use paddler_types::image_url::ImageUrl;

    use crate::decoded_image::DecodedImage;
    use crate::decoded_image_error::DecodedImageError;

    fn create_test_jpeg(width: u32, height: u32) -> Result<Vec<u8>> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Jpeg)?;

        Ok(output_buffer.into_inner())
    }

    fn create_test_tiff(width: u32, height: u32) -> Result<Vec<u8>> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Tiff)?;

        Ok(output_buffer.into_inner())
    }

    fn create_test_png(width: u32, height: u32) -> Result<Vec<u8>> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Png)?;

        Ok(output_buffer.into_inner())
    }

    fn create_test_gif(width: u32, height: u32) -> Result<Vec<u8>> {
        use image::RgbaImage;

        let image_buffer = RgbaImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgba8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Gif)?;

        Ok(output_buffer.into_inner())
    }

    fn create_test_bmp(width: u32, height: u32) -> Result<Vec<u8>> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Bmp)?;

        Ok(output_buffer.into_inner())
    }

    fn load_fixture(filename: &str) -> Result<Vec<u8>> {
        let data = std::fs::read(format!(
            "{}/../fixtures/{filename}",
            env!("CARGO_MANIFEST_DIR"),
        ))?;

        Ok(data)
    }

    #[test]
    fn test_decodes_valid_png_data_uri() -> Result<()> {
        let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47];
        let encoded = BASE64_STANDARD.encode(&png_bytes);
        let image_url = ImageUrl {
            url: format!("data:image/png;base64,{encoded}"),
        };

        let result = DecodedImage::from_data_uri(&image_url)?;

        assert_eq!(result.data, png_bytes);

        Ok(())
    }

    #[test]
    fn test_rejects_remote_url() {
        let image_url = ImageUrl {
            url: "https://example.com/image.png".to_owned(),
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
            url: "data:image/png;base64".to_owned(),
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
            url: "data:image/png;base64,!!!not-valid-base64!!!".to_owned(),
        };

        let result = DecodedImage::from_data_uri(&image_url);

        assert!(matches!(
            result,
            Err(DecodedImageError::InvalidBase64Payload { .. })
        ));
    }

    #[test]
    fn test_prepared_passes_through_small_jpeg() -> Result<()> {
        let jpeg_data = create_test_jpeg(100, 100)?;
        let original_len = jpeg_data.len();
        let decoded_image = DecodedImage { data: jpeg_data };

        let result = decoded_image.prepared_for_inference(1024)?;

        assert_eq!(result.data.len(), original_len);
        Ok(())
    }

    #[test]
    fn test_prepared_passes_through_small_png() -> Result<()> {
        let png_data = create_test_png(100, 100)?;
        let original_len = png_data.len();
        let decoded_image = DecodedImage { data: png_data };

        let result = decoded_image.prepared_for_inference(1024)?;

        assert_eq!(result.data.len(), original_len);
        Ok(())
    }

    #[test]
    fn test_prepared_passes_through_small_gif() -> Result<()> {
        let gif_data = create_test_gif(100, 100)?;
        let original_len = gif_data.len();
        let decoded_image = DecodedImage { data: gif_data };

        let result = decoded_image.prepared_for_inference(1024)?;

        assert_eq!(result.data.len(), original_len);
        Ok(())
    }

    #[test]
    fn test_prepared_passes_through_small_bmp() -> Result<()> {
        let bmp_data = create_test_bmp(100, 100)?;
        let original_len = bmp_data.len();
        let decoded_image = DecodedImage { data: bmp_data };

        let result = decoded_image.prepared_for_inference(1024)?;

        assert_eq!(result.data.len(), original_len);
        Ok(())
    }

    #[test]
    fn test_prepared_converts_small_tiff_to_png() -> Result<()> {
        let tiff_data = create_test_tiff(100, 100)?;
        let decoded_image = DecodedImage { data: tiff_data };

        let result = decoded_image.prepared_for_inference(1024)?;

        let result_format = image::guess_format(&result.data)?;
        assert_eq!(result_format, ImageFormat::Png);
        Ok(())
    }

    #[test]
    fn test_prepared_converts_small_webp_fixture_to_png() -> Result<()> {
        let webp_data = load_fixture("llamas.webp")?;
        let decoded_image = DecodedImage { data: webp_data };

        let result = decoded_image.prepared_for_inference(1024)?;

        let result_format = image::guess_format(&result.data)?;
        assert_eq!(result_format, ImageFormat::Png);

        let result_image = image::load_from_memory(&result.data)?;
        assert_eq!(result_image.width(), 640);
        assert_eq!(result_image.height(), 427);
        Ok(())
    }

    #[test]
    fn test_prepared_rasterizes_small_svg() -> Result<()> {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50">
            <rect width="50" height="50" fill="red"/>
        </svg>"#;
        let decoded_image = DecodedImage {
            data: svg_data.to_vec(),
        };

        let result = decoded_image.prepared_for_inference(1024)?;

        let result_format = image::guess_format(&result.data)?;
        assert_eq!(result_format, ImageFormat::Png);

        let result_image = image::load_from_memory(&result.data)?;
        assert_eq!(result_image.width(), 50);
        assert_eq!(result_image.height(), 50);
        Ok(())
    }

    #[test]
    fn test_prepared_rasterizes_svg_fixture_within_bound() -> Result<()> {
        let svg_data = load_fixture("llamas.svg")?;
        let decoded_image = DecodedImage { data: svg_data };

        let result = decoded_image.prepared_for_inference(320)?;

        let result_format = image::guess_format(&result.data)?;
        let result_image = image::load_from_memory(&result.data)?;

        assert!(result_image.width() <= 320);
        assert!(result_image.height() <= 320);
        assert!(matches!(
            result_format,
            ImageFormat::Png | ImageFormat::Jpeg
        ));
        Ok(())
    }

    #[test]
    fn test_prepared_resizes_oversized_jpeg_to_jpeg() -> Result<()> {
        let jpeg_data = create_test_jpeg(2000, 1500)?;
        let decoded_image = DecodedImage { data: jpeg_data };

        let result = decoded_image.prepared_for_inference(1024)?;

        let result_format = image::guess_format(&result.data)?;
        assert_eq!(result_format, ImageFormat::Jpeg);

        let result_image = image::load_from_memory(&result.data)?;
        assert!(result_image.width() <= 1024);
        assert!(result_image.height() <= 1024);
        Ok(())
    }

    #[test]
    fn test_prepared_preserves_aspect_ratio_on_resize() -> Result<()> {
        let jpeg_data = create_test_jpeg(2000, 1000)?;
        let decoded_image = DecodedImage { data: jpeg_data };

        let result = decoded_image.prepared_for_inference(1000)?;

        let result_image = image::load_from_memory(&result.data)?;
        assert_eq!(result_image.width(), 1000);
        assert_eq!(result_image.height(), 500);
        Ok(())
    }

    #[test]
    fn test_prepared_with_jpg_fixture_within_bound() -> Result<()> {
        let fixture_data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../fixtures/llamas.jpg"
        ))?;

        let original_image = image::load_from_memory(&fixture_data)?;
        assert_eq!(original_image.width(), 640);
        assert_eq!(original_image.height(), 427);

        let decoded_image = DecodedImage { data: fixture_data };
        let result = decoded_image.prepared_for_inference(320)?;

        let result_image = image::load_from_memory(&result.data)?;
        assert_eq!(result_image.width(), 320);
        assert_eq!(result_image.height(), 214);
        Ok(())
    }

    #[test]
    fn test_prepared_rejects_zero_max_dimension() -> Result<()> {
        let png_data = create_test_png(50, 50)?;
        let decoded_image = DecodedImage { data: png_data };

        let result = decoded_image.prepared_for_inference(0);

        assert!(matches!(
            result,
            Err(DecodedImageError::InvalidMaxDimension)
        ));
        Ok(())
    }

    #[test]
    fn test_prepared_rejects_zero_dimension_svg() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="50">
            <rect width="0" height="50" fill="red"/>
        </svg>"#;
        let decoded_image = DecodedImage {
            data: svg_data.to_vec(),
        };

        let result = decoded_image.prepared_for_inference(1024);

        assert!(matches!(
            result,
            Err(DecodedImageError::ConversionFailed { .. })
        ));
    }
}
