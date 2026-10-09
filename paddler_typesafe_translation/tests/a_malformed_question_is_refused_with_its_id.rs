use serde_json::from_value;
use serde_json::json;

use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

#[test]
fn a_malformed_question_is_refused_with_its_id() {
    for malformed_question in [
        json!({"type": "multiple_choice"}),
        json!({"type": "noul", "criteria": "paid on time"}),
    ] {
        let request: SystemOneRequest = from_value(json!({
            "state": "state",
            "questions": {"broken": malformed_question},
        }))
        .expect("the request must parse");

        assert!(matches!(
            request.translate(),
            Err(TypeSafeTranslationError::QuestionMalformed { question_id, .. }) if question_id == "broken"
        ));
    }
}
