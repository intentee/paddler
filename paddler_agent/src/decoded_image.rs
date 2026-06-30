use std::io::Cursor;
use std::str::from_utf8;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use image::DynamicImage;
use image::ImageFormat;
use image::RgbaImage;
use image::guess_format;
use image::imageops::FilterType;
use image::load_from_memory;
use log::info;
use paddler_messaging::image_url::ImageUrl;
use resvg::render;
use resvg::tiny_skia::Pixmap;
use resvg::tiny_skia::Transform;
use resvg::usvg::Options;
use resvg::usvg::Tree as SvgTree;

use crate::decoded_image_error::DecodedImageError;

fn is_svg(data: &[u8]) -> bool {
    let trimmed = match from_utf8(data) {
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

    Ok(target as u32)
}

fn rgba_image_from_raw_buffer(
    width: u32,
    height: u32,
    buffer: Vec<u8>,
) -> Result<RgbaImage, DecodedImageError> {
    RgbaImage::from_raw(width, height, buffer).ok_or_else(|| DecodedImageError::ConversionFailed {
        message: "rasterized pixmap buffer length did not match target dimensions".to_owned(),
    })
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

    let transform = Transform::from_scale(render_scale_x as f32, render_scale_y as f32);

    render(&svg_tree, transform, &mut pixmap.as_mut());

    let rgba = rgba_image_from_raw_buffer(target_width, target_height, pixmap.data().to_vec())?;

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

    let format = guess_format(data).map_err(|err| DecodedImageError::ConversionFailed {
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

    let image = load_from_memory(data).map_err(|err| DecodedImageError::ConversionFailed {
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
    use std::fs::read;
    use std::io::Cursor;
    use std::mem::discriminant;

    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use image::ImageFormat;
    use image::guess_format;
    use image::load_from_memory;
    use paddler_messaging::image_url::ImageUrl;

    use crate::decoded_image::DecodedImage;
    use crate::decoded_image::compute_target_dimension;
    use crate::decoded_image::rgba_image_from_raw_buffer;
    use crate::decoded_image_error::DecodedImageError;

    fn create_test_jpeg(width: u32, height: u32) -> Vec<u8> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Jpeg)
            .unwrap();

        output_buffer.into_inner()
    }

    fn create_test_tiff(width: u32, height: u32) -> Vec<u8> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Tiff)
            .unwrap();

        output_buffer.into_inner()
    }

    fn create_test_png(width: u32, height: u32) -> Vec<u8> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Png)
            .unwrap();

        output_buffer.into_inner()
    }

    fn create_test_gif(width: u32, height: u32) -> Vec<u8> {
        use image::RgbaImage;

        let image_buffer = RgbaImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgba8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Gif)
            .unwrap();

        output_buffer.into_inner()
    }

    fn create_test_bmp(width: u32, height: u32) -> Vec<u8> {
        use image::RgbImage;

        let image_buffer = RgbImage::new(width, height);
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb8(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::Bmp)
            .unwrap();

        output_buffer.into_inner()
    }

    fn create_test_openexr(width: u32, height: u32) -> Vec<u8> {
        use image::Rgb;
        use image::Rgb32FImage;

        let image_buffer = Rgb32FImage::from_pixel(width, height, Rgb([0.25f32, 0.5f32, 0.75f32]));
        let mut output_buffer = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgb32F(image_buffer)
            .write_to(&mut output_buffer, ImageFormat::OpenExr)
            .unwrap();

        output_buffer.into_inner()
    }

    fn load_fixture(filename: &str) -> Vec<u8> {
        read(format!(
            "{}/../fixtures/{filename}",
            env!("CARGO_MANIFEST_DIR"),
        ))
        .unwrap()
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
            url: "https://example.com/image.png".to_owned(),
        };

        let error = DecodedImage::from_data_uri(&image_url).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::RemoteUrlNotSupported),
        );
    }

    #[test]
    fn test_rejects_data_uri_without_comma() {
        let image_url = ImageUrl {
            url: "data:image/png;base64".to_owned(),
        };

        let error = DecodedImage::from_data_uri(&image_url).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::MissingCommaSeparator),
        );
    }

    #[test]
    fn test_rejects_invalid_base64_payload() {
        let image_url = ImageUrl {
            url: "data:image/png;base64,!!!not-valid-base64!!!".to_owned(),
        };

        let error = DecodedImage::from_data_uri(&image_url).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::InvalidBase64Payload {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_passes_through_small_jpeg() {
        let jpeg_data = create_test_jpeg(100, 100);
        let original_len = jpeg_data.len();
        let decoded_image = DecodedImage { data: jpeg_data };

        let result = decoded_image.prepared_for_inference(1024).unwrap();

        assert_eq!(result.data.len(), original_len);
    }

    #[test]
    fn test_prepared_passes_through_small_png() {
        let png_data = create_test_png(100, 100);
        let original_len = png_data.len();
        let decoded_image = DecodedImage { data: png_data };

        let result = decoded_image.prepared_for_inference(1024).unwrap();

        assert_eq!(result.data.len(), original_len);
    }

    #[test]
    fn test_prepared_passes_through_small_gif() {
        let gif_data = create_test_gif(100, 100);
        let original_len = gif_data.len();
        let decoded_image = DecodedImage { data: gif_data };

        let result = decoded_image.prepared_for_inference(1024).unwrap();

        assert_eq!(result.data.len(), original_len);
    }

    #[test]
    fn test_prepared_passes_through_small_bmp() {
        let bmp_data = create_test_bmp(100, 100);
        let original_len = bmp_data.len();
        let decoded_image = DecodedImage { data: bmp_data };

        let result = decoded_image.prepared_for_inference(1024).unwrap();

        assert_eq!(result.data.len(), original_len);
    }

    #[test]
    fn test_prepared_converts_small_tiff_to_png() {
        let tiff_data = create_test_tiff(100, 100);
        let decoded_image = DecodedImage { data: tiff_data };

        let result = decoded_image.prepared_for_inference(1024).unwrap();

        let result_format = guess_format(&result.data).unwrap();
        assert_eq!(result_format, ImageFormat::Png);
    }

    #[test]
    fn test_prepared_converts_small_webp_fixture_to_png() {
        let webp_data = load_fixture("llamas.webp");
        let decoded_image = DecodedImage { data: webp_data };

        let result = decoded_image.prepared_for_inference(1024).unwrap();

        let result_format = guess_format(&result.data).unwrap();
        assert_eq!(result_format, ImageFormat::Png);

        let result_image = load_from_memory(&result.data).unwrap();
        assert_eq!(result_image.width(), 640);
        assert_eq!(result_image.height(), 427);
    }

    #[test]
    fn test_prepared_rasterizes_small_svg() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50">
            <rect width="50" height="50" fill="red"/>
        </svg>"#;
        let decoded_image = DecodedImage {
            data: svg_data.to_vec(),
        };

        let result = decoded_image.prepared_for_inference(1024).unwrap();

        let result_format = guess_format(&result.data).unwrap();
        assert_eq!(result_format, ImageFormat::Png);

        let result_image = load_from_memory(&result.data).unwrap();
        assert_eq!(result_image.width(), 50);
        assert_eq!(result_image.height(), 50);
    }

    #[test]
    fn test_prepared_rasterizes_svg_fixture_within_bound() {
        let svg_data = load_fixture("llamas.svg");
        let decoded_image = DecodedImage { data: svg_data };

        let result = decoded_image.prepared_for_inference(320).unwrap();

        let result_format = guess_format(&result.data).unwrap();
        let result_image = load_from_memory(&result.data).unwrap();

        assert!(result_image.width() <= 320);
        assert!(result_image.height() <= 320);
        assert_eq!(result_format, ImageFormat::Png);
    }

    #[test]
    fn test_prepared_resizes_oversized_jpeg_to_jpeg() {
        let jpeg_data = create_test_jpeg(2000, 1500);
        let decoded_image = DecodedImage { data: jpeg_data };

        let result = decoded_image.prepared_for_inference(1024).unwrap();

        let result_format = guess_format(&result.data).unwrap();
        assert_eq!(result_format, ImageFormat::Jpeg);

        let result_image = load_from_memory(&result.data).unwrap();
        assert!(result_image.width() <= 1024);
        assert!(result_image.height() <= 1024);
    }

    #[test]
    fn test_prepared_preserves_aspect_ratio_on_resize() {
        let jpeg_data = create_test_jpeg(2000, 1000);
        let decoded_image = DecodedImage { data: jpeg_data };

        let result = decoded_image.prepared_for_inference(1000).unwrap();

        let result_image = load_from_memory(&result.data).unwrap();
        assert_eq!(result_image.width(), 1000);
        assert_eq!(result_image.height(), 500);
    }

    #[test]
    fn test_prepared_with_jpg_fixture_within_bound() {
        let fixture_data = load_fixture("llamas.jpg");

        let original_image = load_from_memory(&fixture_data).unwrap();
        assert_eq!(original_image.width(), 640);
        assert_eq!(original_image.height(), 427);

        let decoded_image = DecodedImage { data: fixture_data };
        let result = decoded_image.prepared_for_inference(320).unwrap();

        let result_image = load_from_memory(&result.data).unwrap();
        assert_eq!(result_image.width(), 320);
        assert_eq!(result_image.height(), 214);
    }

    #[test]
    fn test_prepared_rejects_zero_max_dimension() {
        let png_data = create_test_png(50, 50);
        let decoded_image = DecodedImage { data: png_data };

        let error = decoded_image.prepared_for_inference(0).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::InvalidMaxDimension),
        );
    }

    #[test]
    fn test_prepared_rejects_zero_dimension_svg() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="50">
            <rect width="0" height="50" fill="red"/>
        </svg>"#;
        let decoded_image = DecodedImage {
            data: svg_data.to_vec(),
        };

        let error = decoded_image.prepared_for_inference(1024).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_rejects_format_without_reading_support() {
        let dds_header: Vec<u8> = b"DDS \x00\x00\x00\x00".to_vec();
        let decoded_image = DecodedImage { data: dds_header };

        let error = decoded_image.prepared_for_inference(1024).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::UnsupportedFormat {
                format: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_rejects_unrecognized_format_bytes() {
        let unrecognized_bytes: Vec<u8> = b"this is plain text and not any image format".to_vec();
        let decoded_image = DecodedImage {
            data: unrecognized_bytes,
        };

        let error = decoded_image.prepared_for_inference(1024).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_rejects_corrupt_png_body() {
        let mut corrupt_png: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        corrupt_png.extend_from_slice(b"not a real PNG chunk stream");
        let decoded_image = DecodedImage { data: corrupt_png };

        let error = decoded_image.prepared_for_inference(1024).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn rgba_image_from_raw_buffer_rejects_buffer_shorter_than_dimensions() {
        let error = rgba_image_from_raw_buffer(2, 2, vec![0u8; 3])
            .err()
            .unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_compute_target_dimension_rounds_up_within_range() {
        let target = compute_target_dimension(49.2, 1.0).unwrap();

        assert_eq!(target, 50);
    }

    #[test]
    fn test_compute_target_dimension_rejects_below_one() {
        let error = compute_target_dimension(0.0, 1.0).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_compute_target_dimension_rejects_non_finite() {
        let error = compute_target_dimension(f64::INFINITY, 1.0).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_compute_target_dimension_rejects_above_u32_max() {
        let above_u32_max = f64::from(u32::MAX) + 1.0;

        let error = compute_target_dimension(above_u32_max, 1.0).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_rejects_svg_whose_scaled_width_exceeds_u32_max() {
        let svg_data =
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="4295578624" height="1"></svg>"#;
        let decoded_image = DecodedImage {
            data: svg_data.to_vec(),
        };

        let error = decoded_image
            .prepared_for_inference(u32::MAX)
            .err()
            .unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_rejects_svg_whose_scaled_height_exceeds_u32_max() {
        let svg_data =
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="4295578624"></svg>"#;
        let decoded_image = DecodedImage {
            data: svg_data.to_vec(),
        };

        let error = decoded_image
            .prepared_for_inference(u32::MAX)
            .err()
            .unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_rejects_svg_whose_target_pixmap_overflows() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="700000000" height="700000000"></svg>"#;
        let decoded_image = DecodedImage {
            data: svg_data.to_vec(),
        };

        let error = decoded_image
            .prepared_for_inference(600_000_000)
            .err()
            .unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_rejects_float_pixel_format_when_reencoding_to_png() {
        let openexr_data = create_test_openexr(4, 4);
        let decoded_image = DecodedImage { data: openexr_data };

        let error = decoded_image.prepared_for_inference(1024).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ConversionFailed {
                message: String::new(),
            }),
        );
    }

    #[test]
    fn test_prepared_fails_when_resized_dimension_exceeds_jpeg_limit() {
        let png_data = create_test_png(70_001, 2);
        let decoded_image = DecodedImage { data: png_data };

        let error = decoded_image.prepared_for_inference(70_000).err().unwrap();

        assert_eq!(
            discriminant(&error),
            discriminant(&DecodedImageError::ResizeFailed {
                message: String::new(),
            }),
        );
    }
}
