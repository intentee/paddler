use base64::DecodeError;
use image::ImageError;
use resvg::usvg::Error;

#[derive(Debug, thiserror::Error)]
pub enum DecodedImageError {
    #[error("Invalid base64 payload: {0}")]
    InvalidBase64Payload(#[source] DecodeError),

    #[error("Invalid data URI: missing comma separator")]
    MissingCommaSeparator,

    #[error("Data URI metadata {metadata:?} does not end with the ;base64 marker")]
    DataUriIsNotBase64Encoded { metadata: String },

    #[error("Data URI media type {media_type:?} is not an image")]
    DataUriMediaTypeIsNotAnImage { media_type: String },

    #[error("Data URI image media type has no subtype")]
    DataUriImageSubtypeIsEmpty,

    #[error("Data URI media type {media_type:?} has parameters, which images do not take")]
    DataUriMediaTypeHasParameters { media_type: String },

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
