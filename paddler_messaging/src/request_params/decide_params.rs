use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecideParams {
    pub request: DecisionRequest,
}
