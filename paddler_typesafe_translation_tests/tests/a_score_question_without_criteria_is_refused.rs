use serde_json::json;

use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;
use paddler_typesafe_translation_tests::system_one_request_with_question::system_one_request_with_question;

#[test]
fn a_score_question_without_criteria_is_refused() {
    assert!(matches!(
        system_one_request_with_question(&json!({"type": "score", "criteria": []}))
            .expect("the request must parse")
            .translate(),
        Err(TypeSafeTranslationError::ScoreCriteriaOutOfRange {
            criteria_count: 0,
            question_id,
        }) if question_id == "question"
    ));
}
