use anyhow::Error as AnyhowError;
use reqwest::Error as ReqwestError;
use serde_json::Error as JsonError;
use thiserror::Error as ThisError;
use tokio_tungstenite::tungstenite::Error as WsError;
use url::ParseError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] ReqwestError),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] WsError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] JsonError),

    #[error("URL parse error: {0}")]
    Url(#[from] ParseError),

    #[error("WebSocket pool exhausted: no available connections")]
    PoolExhausted,

    #[error("Request {request_id} failed: connection dropped")]
    ConnectionDropped { request_id: String },

    #[error("Server returned error: {message}")]
    Server { code: i32, message: String },

    #[error("{0}")]
    Other(String),
}

impl From<AnyhowError> for Error {
    fn from(err: AnyhowError) -> Self {
        Error::Other(err.to_string())
    }
}

pub type Result<TValue> = std::result::Result<TValue, Error>;
