use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde_json::Value;

pub struct TypeSafeApiResponse {
    pub body: Value,
    pub headers: HeaderMap,
    pub status: StatusCode,
}
