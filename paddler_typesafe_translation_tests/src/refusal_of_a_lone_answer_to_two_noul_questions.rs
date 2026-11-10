use serde_json::Value;
use serde_json::json;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::translated_system_one_request::TranslatedSystemOneRequest;

const SUMMARY: DecisionSummary = DecisionSummary {
    input_tokens: 1,
    processing_milliseconds: 1,
};

pub fn refusal_of_a_lone_answer_to_two_noul_questions(
    id: &str,
    probabilities: Vec<f32>,
) -> Option<String> {
    SystemOneRequest {
        model: "test".to_owned(),
        questions: [
            ("first".to_owned(), json!({"type": "noul"})),
            ("second".to_owned(), json!({"type": "noul"})),
        ]
        .into_iter()
        .collect(),
        state: Value::from("state"),
    }
    .translate()
    .and_then(
        |TranslatedSystemOneRequest {
             response_layout, ..
         }| {
            response_layout.respond(
                &[DecisionAnswer {
                    id: id.to_owned(),
                    probabilities,
                }],
                &SUMMARY,
            )
        },
    )
    .err()
    .as_ref()
    .map(ToString::to_string)
}
