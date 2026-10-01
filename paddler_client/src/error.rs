use std::result::Result as StdResult;
use std::string::FromUtf8Error;

use reqwest::Error as ReqwestError;
use reqwest::StatusCode;
use serde_json::Error as SerdeJsonError;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use url::ParseError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] ReqwestError),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] WebSocketError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] SerdeJsonError),

    #[error("Failed to parse NDJSON line: {line}")]
    NdjsonLineParseFailed {
        line: String,
        #[source]
        source: SerdeJsonError,
    },

    #[error("Stream produced a line that is not valid UTF-8")]
    NonUtf8StreamLine {
        #[source]
        source: FromUtf8Error,
    },

    #[error("URL parse error: {0}")]
    Url(#[from] ParseError),

    #[error("Failed to connect to {url}")]
    Connect {
        url: String,
        #[source]
        source: ReqwestError,
    },

    #[error("Service at {url} is unavailable: {message}")]
    ServiceUnavailable { message: String, url: String },

    #[error("Request to {url} returned unexpected status {status}: {message}")]
    UnexpectedResponseStatus {
        message: String,
        status: StatusCode,
        url: String,
    },

    #[error("Cannot use {url} as an inference socket URL: its scheme cannot be set to {scheme}")]
    InferenceSocketUrlSchemeRejected { scheme: String, url: String },

    #[error("Request {request_id} failed: connection dropped")]
    ConnectionDropped { request_id: String },

    #[error("Request to {url} was cancelled before the server responded")]
    RequestCancelled { url: String },

    #[error("Inference request {request_id} is already in flight on this connection")]
    InferenceRequestIdInFlight { request_id: String },

    #[error("Inference request {request_id} was cancelled before it reached the server")]
    InferenceRequestCancelled { request_id: String },

    #[error("Request {request_id} received a message that could not be decoded")]
    UndecodableInferenceMessage {
        request_id: String,
        #[source]
        source: SerdeJsonError,
    },
}

pub type Result<TValue> = StdResult<TValue, Error>;
