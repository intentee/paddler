#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Failed to parse NDJSON line: {line}")]
    NdjsonLineParseFailed {
        line: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Stream produced a line that is not valid UTF-8")]
    NonUtf8StreamLine {
        #[source]
        source: std::string::FromUtf8Error,
    },

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Failed to connect to {url}")]
    Connect {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Service at {url} is unavailable")]
    ServiceUnavailable { url: String },

    #[error("Request to {url} returned unexpected status {status}")]
    UnexpectedResponseStatus {
        status: reqwest::StatusCode,
        url: String,
    },

    #[error("Cannot use {url} as an inference socket URL: its scheme cannot be set to {scheme}")]
    InferenceSocketUrlSchemeRejected { scheme: String, url: String },

    #[error("Connection slot is empty after connection attempt")]
    ConnectionSlotEmpty,

    #[error("Request {request_id} failed: connection dropped")]
    ConnectionDropped { request_id: String },

    #[error("Request {request_id} failed to send after reconnecting")]
    ReconnectionFailed {
        request_id: String,
        #[source]
        source: Box<Self>,
    },
}

pub type Result<TValue> = std::result::Result<TValue, Error>;
