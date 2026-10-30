use serde_json::json;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_summary::DecisionSummary;

use crate::system_one_request_with_questions::system_one_request_with_questions;

const SUMMARY: DecisionSummary = DecisionSummary {
    input_tokens: 1,
    processing_milliseconds: 1,
};

pub fn refusal_of_a_lone_answer_to_two_noul_questions(
    id: &str,
    probabilities: Vec<f32>,
) -> Option<String> {
    system_one_request_with_questions(
        json!({"first": {"type": "noul"}, "second": {"type": "noul"}}),
    )
    .expect("the request must parse")
    .translate()
    .expect("the request must translate")
    .response_layout
    .respond(
        &[DecisionAnswer {
            id: id.to_owned(),
            probabilities,
        }],
        &SUMMARY,
    )
    .err()
    .as_ref()
    .map(ToString::to_string)
}
