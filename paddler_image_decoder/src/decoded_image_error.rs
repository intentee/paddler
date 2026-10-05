use base64::DecodeError;
use image::ImageError;
use resvg::usvg::Error;

#[derive(Debug, thiserror::Error)]
pub enum DecodedImageError {
    #[error("Invalid base64 payload: {0}")]
    InvalidBase64Payload(#[source] DecodeError),

    #[error("Invalid data URI: missing comma separator")]
    MissingCommaSeparator,

    #[error("Unrecognized image format: {raster_format_error}; it is not an SVG either")]
    UnrecognizedFormat {
        raster_format_error: ImageError,
        #[source]
        svg_error: Error,
    },

    #[error("Unsupported image format: {format}")]
    UnsupportedFormat { format: String },

    #[error("Failed to decode image pixels: {0}")]
    PixelDecodingFailed(#[source] ImageError),

    #[error("SVG dimension {dimension} is out of valid range")]
    SvgDimensionOutOfRange { dimension: f64 },

    #[error("Failed to allocate a {width}x{height} pixmap for SVG rasterization")]
    SvgPixmapAllocationFailed { width: u32, height: u32 },

    #[error(
        "Remote image URLs are not supported. Use base64 data URIs (data:image/...;base64,...) instead."
    )]
    RemoteUrlNotSupported,
}
