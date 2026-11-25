use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use serde_json::json;

use paddler_typesafe_translation_tests::system_one_request_with_questions::system_one_request_with_questions;

#[test]
fn a_noul_question_with_null_criteria_describes_neither_outcome() {
    let translated =
        system_one_request_with_questions(&json!({"paid": {"type": "noul", "criteria": null}}))
            .expect("the request must parse")
            .translate()
            .expect("the request must translate");

    assert!(matches!(
        translated.decide_params.request.questions.get("paid"),
        Some(DecisionQuestion::Noul { criteria, .. }) if criteria.is_empty()
    ));
}
