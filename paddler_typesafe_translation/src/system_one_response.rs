use indexmap::IndexMap;
use serde::Serialize;

use crate::system_one_answer::SystemOneAnswer;
use crate::system_one_usage::SystemOneUsage;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SystemOneResponse {
    pub model: String,
    pub answers: IndexMap<String, SystemOneAnswer>,
    pub usage: SystemOneUsage,
    pub latency_ms: u128,
}
