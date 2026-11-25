use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::json;

use crate::sample_decision::SAMPLE_DECISION_STATE;
use crate::sample_decision::sample_decision;

#[must_use]
pub fn ticket_decision(ticket_number: usize) -> DecisionRequest {
    DecisionRequest {
        state: json!(format!("Ticket {ticket_number}: {SAMPLE_DECISION_STATE}")),
        ..sample_decision()
    }
}
