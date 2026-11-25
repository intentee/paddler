use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::json;

use crate::sample_decision::SAMPLE_DECISION_STATE;
use crate::sample_decision::sample_decision;

#[must_use]
pub fn repeated_state_decision(state_repetitions: usize) -> DecisionRequest {
    DecisionRequest {
        state: json!(SAMPLE_DECISION_STATE.repeat(state_repetitions)),
        ..sample_decision()
    }
}
