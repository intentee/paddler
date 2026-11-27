use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::Value;
use serde_json::json;

use crate::choice_question::choice_question;

pub const SAMPLE_DECISION_STATE: &str = "Ada paid the invoice for order 1042 two days late and wrote that the delivery was slow, but the support team was friendly.";

#[must_use]
pub fn sample_decision() -> DecisionRequest {
    DecisionRequest {
        images: Vec::new(),
        questions: IndexMap::from([
            (
                "paid".to_owned(),
                DecisionQuestion::Noul {
                    criteria: IndexMap::new(),
                    instructions: json!("Was the invoice paid?"),
                },
            ),
            (
                "sentiment".to_owned(),
                DecisionQuestion::Choice {
                    criteria: IndexMap::from([
                        ("positive".to_owned(), Value::Null),
                        ("neutral".to_owned(), Value::Null),
                        ("negative".to_owned(), json!("the customer complains")),
                    ]),
                    instructions: json!("How does the customer feel?"),
                },
            ),
            (
                "urgency".to_owned(),
                choice_question(
                    "How urgent is the follow-up?",
                    &["none", "low", "medium", "high"],
                ),
            ),
        ]),
        state: json!(SAMPLE_DECISION_STATE),
    }
}
