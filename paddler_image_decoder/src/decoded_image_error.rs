#[derive(Debug, thiserror::Error)]
pub enum DecodedImageError {
    #[error("Invalid base64 payload: {0}")]
    InvalidBase64Payload(#[source] base64::DecodeError),

    #[error("Invalid data URI: missing comma separator")]
    MissingCommaSeparator,

    #[error("max_dimension must be greater than zero")]
    InvalidMaxDimension,

    #[error("Unrecognized image format: {0}")]
    UnrecognizedFormat(#[source] image::ImageError),

    #[error("Unsupported image format: {format}")]
    UnsupportedFormat { format: String },

    #[error("Failed to decode image pixels: {0}")]
    PixelDecodingFailed(#[source] image::ImageError),

    #[error("Failed to parse SVG: {0}")]
    SvgParsingFailed(#[source] resvg::usvg::Error),

    #[error("SVG dimension {dimension} is out of valid range")]
    SvgDimensionOutOfRange { dimension: f64 },

    #[error("Failed to allocate a {width}x{height} pixmap for SVG rasterization")]
    SvgPixmapAllocationFailed { width: u32, height: u32 },

    #[error(
        "Remote image URLs are not supported. Use base64 data URIs (data:image/...;base64,...) instead."
    )]
    RemoteUrlNotSupported,
}
