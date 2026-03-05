#[derive(Debug, thiserror::Error)]
pub enum DecodedImageError {
    #[error("Invalid base64 payload: {message}")]
    InvalidBase64Payload { message: String },

    #[error("Invalid data URI: missing comma separator")]
    MissingCommaSeparator,

    #[error(
        "Remote image URLs are not supported. Use base64 data URIs (data:image/...;base64,...) instead."
    )]
    RemoteUrlNotSupported,
}
