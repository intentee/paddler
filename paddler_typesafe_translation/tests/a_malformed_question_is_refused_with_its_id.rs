use serde_json::from_value;
use serde_json::json;

use paddler_typesafe_translation::system_one_question::SystemOneQuestion;
use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

#[test]
fn a_malformed_question_is_refused_with_its_id() {
    for malformed_question in [
        json!({"type": "multiple_choice"}),
        json!({"type": "noul", "criteria": "paid on time"}),
    ] {
        let question_error = from_value::<SystemOneQuestion>(malformed_question.clone())
            .expect_err("the question must be malformed");
        let request: SystemOneRequest = from_value(json!({
            "state": "state",
            "questions": {"broken": malformed_question},
        }))
        .expect("the request must parse");

        assert_eq!(
            request.translate().err().as_ref().map(ToString::to_string),
            Some(
                TypeSafeTranslationError::QuestionMalformed {
                    question_id: "broken".to_owned(),
                    source: question_error,
                }
                .to_string()
            )
        );
    }
}
