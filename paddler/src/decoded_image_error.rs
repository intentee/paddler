#[derive(Debug, thiserror::Error)]
pub enum DecodedImageError {
    #[error("Invalid base64 payload: {message}")]
    InvalidBase64Payload { message: String },

    #[error("Invalid data URI: missing comma separator")]
    MissingCommaSeparator,

    #[error("Failed to convert image to PNG: {message}")]
    ConversionFailed { message: String },

    #[error("max_dimension must be greater than zero")]
    InvalidMaxDimension,

    #[error("Failed to resize image: {message}")]
    ResizeFailed { message: String },

    #[error("Unsupported image format: {format}")]
    UnsupportedFormat { format: String },

    #[error(
        "Remote image URLs are not supported. Use base64 data URIs (data:image/...;base64,...) instead."
    )]
    RemoteUrlNotSupported,
}
