use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KevParityRecord {
    pub probabilities: Vec<Vec<f64>>,
    pub request: DecisionRequest,
}
