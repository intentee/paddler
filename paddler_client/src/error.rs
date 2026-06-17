#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("inference URL scheme '{scheme}' is not supported; expected http, https, ws, or wss")]
    InferenceSocketUnsupportedScheme { scheme: String },

    #[error("Request {request_id} failed: connection dropped")]
    ConnectionDropped { request_id: String },

    #[error("CORS preflight response did not include an Access-Control-Allow-Origin header")]
    CorsAllowOriginMissing,

    #[error("CORS Access-Control-Allow-Origin header was not valid ASCII: {source}")]
    CorsAllowOriginNotAscii {
        #[source]
        source: reqwest::header::ToStrError,
    },
}

pub type Result<TValue> = std::result::Result<TValue, Error>;
