#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("WebSocket pool exhausted: no available connections")]
    PoolExhausted,

    #[error("Request {request_id} failed: connection dropped")]
    ConnectionDropped { request_id: String },

    #[error("Server returned error: {message}")]
    Server { code: i32, message: String },

    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}

pub type Result<TValue> = std::result::Result<TValue, Error>;
