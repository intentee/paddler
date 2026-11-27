use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use serde_json::json;

use paddler_typesafe_translation_tests::system_one_request_with_questions::system_one_request_with_questions;

#[test]
fn a_question_without_instructions_is_asked_with_empty_instructions() {
    let translated = system_one_request_with_questions(&json!({
        "omitted": {"type": "noul"},
        "null": {"type": "noul", "instructions": null},
    }))
    .expect("the request must parse")
    .translate()
    .expect("the request must translate");

    assert!(
        translated
            .decide_params
            .request
            .questions
            .values()
            .all(|question| matches!(
                question,
                DecisionQuestion::Noul { instructions, .. } if *instructions == json!("")
            ))
    );
}
