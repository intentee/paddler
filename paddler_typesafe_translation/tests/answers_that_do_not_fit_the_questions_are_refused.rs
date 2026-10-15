use serde_json::from_value;
use serde_json::json;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::system_one_response_layout::SystemOneResponseLayout;
use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

fn answer(id: &str, probabilities: Vec<f32>) -> DecisionAnswer {
    DecisionAnswer {
        id: id.to_owned(),
        probabilities,
    }
}

const SUMMARY: DecisionSummary = DecisionSummary {
    input_tokens: 1,
    processing_milliseconds: 1,
};

#[test]
fn answers_that_do_not_fit_the_questions_are_refused() {
    let response_layout: SystemOneResponseLayout = from_value::<SystemOneRequest>(json!({
        "state": "state",
        "questions": {"first": {"type": "noul"}, "second": {"type": "noul"}},
    }))
    .expect("the request must parse")
    .translate()
    .expect("the request must translate")
    .response_layout;

    assert_eq!(
        response_layout
            .respond(&[answer("first", vec![0.5, 0.5])], &SUMMARY)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TypeSafeTranslationError::AnswerMissing {
                question_id: "second".to_owned(),
            }
            .to_string()
        )
    );
    assert_eq!(
        response_layout
            .respond(&[answer("second", vec![0.5, 0.5])], &SUMMARY)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TypeSafeTranslationError::AnswerOutOfOrder {
                expected_question_id: "first".to_owned(),
                received_question_id: "second".to_owned(),
            }
            .to_string()
        )
    );
    assert_eq!(
        response_layout
            .respond(&[answer("first", vec![1.0])], &SUMMARY)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TypeSafeTranslationError::AnswerProbabilitiesMismatch {
                expected: 2,
                question_id: "first".to_owned(),
                received: 1,
            }
            .to_string()
        )
    );
}
