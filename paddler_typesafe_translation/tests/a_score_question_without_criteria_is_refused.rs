use serde_json::json;

use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

use crate::system_one_request_with_question::system_one_request_with_question;

#[test]
fn a_score_question_without_criteria_is_refused() {
    assert_eq!(
        system_one_request_with_question(json!({"type": "score", "criteria": []}))
            .expect("the request must parse")
            .translate()
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TypeSafeTranslationError::ScoreCriteriaOutOfRange {
                criteria_count: 0,
                question_id: "question".to_owned(),
            }
            .to_string()
        )
    );
}
