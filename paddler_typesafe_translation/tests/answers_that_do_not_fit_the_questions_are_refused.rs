use serde_json::from_value;
use serde_json::json;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::translated_system_one_request::TranslatedSystemOneRequest;
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
    let translated: TranslatedSystemOneRequest = from_value::<SystemOneRequest>(json!({
        "state": "state",
        "questions": {"first": {"type": "noul"}, "second": {"type": "noul"}},
    }))
    .expect("the request must parse")
    .translate()
    .expect("the request must translate");

    assert!(matches!(
        translated.respond(&[answer("first", vec![0.5, 0.5])], &SUMMARY),
        Err(TypeSafeTranslationError::AnswerMissing { question_id }) if question_id == "second"
    ));
    assert!(matches!(
        translated.respond(&[answer("second", vec![0.5, 0.5])], &SUMMARY),
        Err(TypeSafeTranslationError::AnswerOutOfOrder { expected_question_id, .. })
            if expected_question_id == "first"
    ));
    assert!(matches!(
        translated.respond(&[answer("first", vec![1.0])], &SUMMARY),
        Err(TypeSafeTranslationError::AnswerProbabilitiesMismatch {
            expected,
            received,
            ..
        }) if expected == 2 && received == 1
    ));
}
