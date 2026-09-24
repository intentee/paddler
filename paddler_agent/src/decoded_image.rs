use std::str::from_utf8;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use image::DynamicImage;
use image::guess_format;
use image::imageops::FilterType;
use image::load_from_memory_with_format;
use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdBitmapError;
use log::info;
use paddler_messaging::image_url::ImageUrl;
use resvg::render;
use resvg::tiny_skia::Pixmap;
use resvg::tiny_skia::Transform;
use resvg::usvg::Options;
use resvg::usvg::Tree as SvgTree;

use crate::decoded_image_error::DecodedImageError;

const RGBA_BYTES_PER_PIXEL: usize = 4;
const RGB_BYTES_PER_PIXEL: usize = 3;

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
        return Err(DecodedImageError::SvgDimensionOutOfRange { dimension: target });
    }

    Ok(target as u32)
}

fn rasterize_svg(data: &[u8], max_dimension: u32) -> Result<DecodedImage, DecodedImageError> {
    let svg_tree = SvgTree::from_data(data, &Options::default())
        .map_err(DecodedImageError::SvgParsingFailed)?;

    let svg_size = svg_tree.size();
    let svg_width = f64::from(svg_size.width());
    let svg_height = f64::from(svg_size.height());
    let max_dim = f64::from(max_dimension);

    let scale = (max_dim / svg_width).min(max_dim / svg_height).min(1.0);

    let target_width = compute_target_dimension(svg_width, scale)?;
    let target_height = compute_target_dimension(svg_height, scale)?;

    let mut pixmap = Pixmap::new(target_width, target_height).ok_or(
        DecodedImageError::SvgPixmapAllocationFailed {
            width: target_width,
            height: target_height,
        },
    )?;

    let render_scale_x = f64::from(target_width) / svg_width;
    let render_scale_y = f64::from(target_height) / svg_height;

    let transform = Transform::from_scale(render_scale_x as f32, render_scale_y as f32);

    render(&svg_tree, transform, &mut pixmap.as_mut());

    Ok(DecodedImage {
        height: target_height,
        rgb_pixels: pixmap
            .data()
            .chunks_exact(RGBA_BYTES_PER_PIXEL)
            .flat_map(|rgba_pixel| &rgba_pixel[..RGB_BYTES_PER_PIXEL])
            .copied()
            .collect(),
        width: target_width,
    })
}

fn decode_raster_image(data: &[u8]) -> Result<DynamicImage, DecodedImageError> {
    let format = guess_format(data).map_err(DecodedImageError::UnrecognizedFormat)?;

    if !format.reading_enabled() {
        return Err(DecodedImageError::UnsupportedFormat {
            format: format!("{format:?}"),
        });
    }

    load_from_memory_with_format(data, format).map_err(DecodedImageError::PixelDecodingFailed)
}

fn fit_raster_image(image: DynamicImage, max_dimension: u32) -> DecodedImage {
    let fitted_image = if image.width() > max_dimension || image.height() > max_dimension {
        image.resize(max_dimension, max_dimension, FilterType::Lanczos3)
    } else {
        image
    };

    let rgb_image = fitted_image.into_rgb8();

    DecodedImage {
        height: rgb_image.height(),
        width: rgb_image.width(),
        rgb_pixels: rgb_image.into_raw(),
    }
}

fn decode_data_uri_payload(image_url: &ImageUrl) -> Result<Vec<u8>, DecodedImageError> {
    let after_data = image_url
        .url
        .strip_prefix("data:")
        .ok_or(DecodedImageError::RemoteUrlNotSupported)?;

    let (_metadata, encoded_data) = after_data
        .split_once(',')
        .ok_or(DecodedImageError::MissingCommaSeparator)?;

    BASE64_STANDARD
        .decode(encoded_data)
        .map_err(DecodedImageError::InvalidBase64Payload)
}

#[derive(Debug)]
pub struct DecodedImage {
    pub height: u32,
    pub rgb_pixels: Vec<u8>,
    pub width: u32,
}

impl DecodedImage {
    pub fn from_data_uri(
        image_url: &ImageUrl,
        max_dimension: u32,
    ) -> Result<Self, DecodedImageError> {
        if max_dimension == 0 {
            return Err(DecodedImageError::InvalidMaxDimension);
        }

        let encoded_image = decode_data_uri_payload(image_url)?;

        if is_svg(&encoded_image) {
            info!("Rasterizing SVG (max_dimension: {max_dimension})");

            return rasterize_svg(&encoded_image, max_dimension);
        }

        Ok(fit_raster_image(
            decode_raster_image(&encoded_image)?,
            max_dimension,
        ))
    }

    pub fn into_bitmap(self) -> Result<MtmdBitmap, MtmdBitmapError> {
        MtmdBitmap::from_image_data(self.width, self.height, &self.rgb_pixels)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::read;
    use std::io::Cursor;

    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use image::DynamicImage;
    use image::ImageFormat;
    use image::Rgb;
    use image::Rgb32FImage;
    use image::RgbImage;
    use image::RgbaImage;
    use paddler_messaging::image_url::ImageUrl;

    use crate::decoded_image::DecodedImage;
    use crate::decoded_image::compute_target_dimension;
    use crate::decoded_image_error::DecodedImageError;

    fn encode_image(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut output_buffer = Cursor::new(Vec::new());

        image.write_to(&mut output_buffer, format).unwrap();

        output_buffer.into_inner()
    }

    fn create_rgb_image(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        encode_image(
            DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, Rgb([10, 20, 30]))),
            format,
        )
    }

    fn load_fixture(filename: &str) -> Vec<u8> {
        read(format!(
            "{}/../fixtures/{filename}",
            env!("CARGO_MANIFEST_DIR"),
        ))
        .unwrap()
    }

    fn data_uri(encoded_image: &[u8]) -> ImageUrl {
        ImageUrl {
            url: format!(
                "data:image/png;base64,{}",
                BASE64_STANDARD.encode(encoded_image)
            ),
        }
    }

    fn decode(encoded_image: &[u8], max_dimension: u32) -> Result<DecodedImage, DecodedImageError> {
        DecodedImage::from_data_uri(&data_uri(encoded_image), max_dimension)
    }

    fn assert_decodes_small_image_without_resizing(format: ImageFormat) {
        let decoded_image = decode(&create_rgb_image(100, 60, format), 1024).unwrap();

        assert_eq!(decoded_image.width, 100);
        assert_eq!(decoded_image.height, 60);
        assert_eq!(decoded_image.rgb_pixels.len(), 100 * 60 * 3);
    }

    #[test]
    fn decodes_png_pixels_as_rgb() {
        let decoded_image = decode(&create_rgb_image(2, 2, ImageFormat::Png), 1024).unwrap();

        assert_eq!(decoded_image.rgb_pixels, [10, 20, 30].repeat(4));
    }

    #[test]
    fn rejects_remote_url() {
        let image_url = ImageUrl {
            url: "https://example.com/image.png".to_owned(),
        };

        let error = DecodedImage::from_data_uri(&image_url, 1024).err().unwrap();

        assert!(matches!(error, DecodedImageError::RemoteUrlNotSupported));
    }

    #[test]
    fn rejects_data_uri_without_comma() {
        let image_url = ImageUrl {
            url: "data:image/png;base64".to_owned(),
        };

        let error = DecodedImage::from_data_uri(&image_url, 1024).err().unwrap();

        assert!(matches!(error, DecodedImageError::MissingCommaSeparator));
    }

    #[test]
    fn rejects_invalid_base64_payload() {
        let image_url = ImageUrl {
            url: "data:image/png;base64,!!!not-valid-base64!!!".to_owned(),
        };

        let error = DecodedImage::from_data_uri(&image_url, 1024).err().unwrap();

        assert!(matches!(error, DecodedImageError::InvalidBase64Payload(_)));
    }

    #[test]
    fn decodes_small_jpeg_without_resizing() {
        assert_decodes_small_image_without_resizing(ImageFormat::Jpeg);
    }

    #[test]
    fn decodes_small_png_without_resizing() {
        assert_decodes_small_image_without_resizing(ImageFormat::Png);
    }

    #[test]
    fn decodes_small_bmp_without_resizing() {
        assert_decodes_small_image_without_resizing(ImageFormat::Bmp);
    }

    #[test]
    fn decodes_small_tiff_without_resizing() {
        assert_decodes_small_image_without_resizing(ImageFormat::Tiff);
    }

    #[test]
    fn decodes_small_gif_without_resizing() {
        let gif_data = encode_image(
            DynamicImage::ImageRgba8(RgbaImage::new(100, 60)),
            ImageFormat::Gif,
        );

        let decoded_image = decode(&gif_data, 1024).unwrap();

        assert_eq!(decoded_image.width, 100);
        assert_eq!(decoded_image.height, 60);
    }

    #[test]
    fn decodes_webp_fixture_without_resizing() {
        let decoded_image = decode(&load_fixture("llamas.webp"), 1024).unwrap();

        assert_eq!(decoded_image.width, 640);
        assert_eq!(decoded_image.height, 427);
    }

    #[test]
    fn converts_float_pixel_format_to_rgb() {
        let openexr_data = encode_image(
            DynamicImage::ImageRgb32F(Rgb32FImage::from_pixel(
                4,
                4,
                Rgb([0.25f32, 0.5f32, 0.75f32]),
            )),
            ImageFormat::OpenExr,
        );

        let decoded_image = decode(&openexr_data, 1024).unwrap();

        assert_eq!(decoded_image.rgb_pixels.len(), 4 * 4 * 3);
    }

    #[test]
    fn rasterizes_small_svg() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50">
            <rect width="50" height="50" fill="red"/>
        </svg>"#;

        let decoded_image = decode(svg_data, 1024).unwrap();

        assert_eq!(decoded_image.width, 50);
        assert_eq!(decoded_image.height, 50);
        assert_eq!(&decoded_image.rgb_pixels[0..3], &[255, 0, 0]);
    }

    #[test]
    fn rasterizes_svg_fixture_within_bound() {
        let decoded_image = decode(&load_fixture("llamas.svg"), 320).unwrap();

        assert!(decoded_image.width <= 320);
        assert!(decoded_image.height <= 320);
    }

    #[test]
    fn resizes_oversized_jpeg_within_bound() {
        let decoded_image = decode(&create_rgb_image(2000, 1500, ImageFormat::Jpeg), 1024).unwrap();

        assert!(decoded_image.width <= 1024);
        assert!(decoded_image.height <= 1024);
    }

    #[test]
    fn preserves_aspect_ratio_on_resize() {
        let decoded_image = decode(&create_rgb_image(2000, 1000, ImageFormat::Jpeg), 1000).unwrap();

        assert_eq!(decoded_image.width, 1000);
        assert_eq!(decoded_image.height, 500);
    }

    #[test]
    fn resizes_jpg_fixture_within_bound() {
        let decoded_image = decode(&load_fixture("llamas.jpg"), 320).unwrap();

        assert_eq!(decoded_image.width, 320);
        assert_eq!(decoded_image.height, 214);
    }

    #[test]
    fn rejects_zero_max_dimension() {
        let error = decode(&create_rgb_image(50, 50, ImageFormat::Png), 0)
            .err()
            .unwrap();

        assert!(matches!(error, DecodedImageError::InvalidMaxDimension));
    }

    #[test]
    fn rejects_zero_dimension_svg() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="50">
            <rect width="0" height="50" fill="red"/>
        </svg>"#;

        let error = decode(svg_data, 1024).err().unwrap();

        assert!(matches!(error, DecodedImageError::SvgParsingFailed(_)));
    }

    #[test]
    fn rejects_format_without_reading_support() {
        let error = decode(b"DDS \x00\x00\x00\x00", 1024).err().unwrap();

        assert!(matches!(error, DecodedImageError::UnsupportedFormat { .. }));
    }

    #[test]
    fn rejects_unrecognized_format_bytes() {
        let error = decode(b"this is plain text and not any image format", 1024)
            .err()
            .unwrap();

        assert!(matches!(error, DecodedImageError::UnrecognizedFormat(_)));
    }

    #[test]
    fn rejects_corrupt_png_body() {
        let mut corrupt_png: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        corrupt_png.extend_from_slice(b"not a real PNG chunk stream");

        let error = decode(&corrupt_png, 1024).err().unwrap();

        assert!(matches!(error, DecodedImageError::PixelDecodingFailed(_)));
    }

    #[test]
    fn compute_target_dimension_rounds_up_within_range() {
        assert_eq!(compute_target_dimension(49.2, 1.0).unwrap(), 50);
    }

    #[test]
    fn compute_target_dimension_rejects_below_one() {
        let error = compute_target_dimension(0.0, 1.0).err().unwrap();

        assert!(matches!(
            error,
            DecodedImageError::SvgDimensionOutOfRange { .. }
        ));
    }

    #[test]
    fn compute_target_dimension_rejects_non_finite() {
        let error = compute_target_dimension(f64::INFINITY, 1.0).err().unwrap();

        assert!(matches!(
            error,
            DecodedImageError::SvgDimensionOutOfRange { .. }
        ));
    }

    #[test]
    fn compute_target_dimension_rejects_above_u32_max() {
        let error = compute_target_dimension(f64::from(u32::MAX) + 1.0, 1.0)
            .err()
            .unwrap();

        assert!(matches!(
            error,
            DecodedImageError::SvgDimensionOutOfRange { .. }
        ));
    }

    #[test]
    fn rejects_svg_whose_scaled_width_exceeds_u32_max() {
        let svg_data =
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="4295578624" height="1"></svg>"#;

        let error = decode(svg_data, u32::MAX).err().unwrap();

        assert!(matches!(
            error,
            DecodedImageError::SvgDimensionOutOfRange { .. }
        ));
    }

    #[test]
    fn rejects_svg_whose_scaled_height_exceeds_u32_max() {
        let svg_data =
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="4295578624"></svg>"#;

        let error = decode(svg_data, u32::MAX).err().unwrap();

        assert!(matches!(
            error,
            DecodedImageError::SvgDimensionOutOfRange { .. }
        ));
    }

    #[test]
    fn rejects_svg_whose_target_pixmap_overflows() {
        let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="700000000" height="700000000"></svg>"#;

        let error = decode(svg_data, 600_000_000).err().unwrap();

        assert!(matches!(
            error,
            DecodedImageError::SvgPixmapAllocationFailed { .. }
        ));
    }

    #[test]
    fn converts_decoded_pixels_into_bitmap_with_matching_dimensions() {
        let decoded_image = decode(&create_rgb_image(8, 4, ImageFormat::Png), 1024).unwrap();

        let bitmap = decoded_image.into_bitmap().unwrap();

        assert_eq!(bitmap.nx(), 8);
        assert_eq!(bitmap.ny(), 4);
    }
}
