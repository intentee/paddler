use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::system_one_usage::SystemOneUsage;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SystemOneResponse {
    pub model: String,
    pub answers: Map<String, Value>,
    pub usage: SystemOneUsage,
    pub latency_ms: u64,
}
