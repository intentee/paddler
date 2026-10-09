use serde_json::Error as JsonError;
use serde_json::Map;
use serde_json::Value;
use serde_json::from_value;
use serde_json::json;

use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

const TOO_MANY_CRITERIA: usize = 256;

fn request_with_question(question: Value) -> Result<SystemOneRequest, JsonError> {
    from_value(json!({"state": "state", "questions": {"question": question}}))
}

#[test]
fn questions_with_too_many_or_no_criteria_are_refused() {
    let too_many_choices: Map<String, Value> = (0..TOO_MANY_CRITERIA)
        .map(|option| (format!("option-{option}"), Value::Null))
        .collect();

    assert!(matches!(
        request_with_question(json!({"type": "choice", "criteria": too_many_choices}))
            .expect("the request must parse")
            .translate(),
        Err(TypeSafeTranslationError::ChoiceCriteriaOutOfRange {
            criteria_count,
            ..
        }) if criteria_count == TOO_MANY_CRITERIA
    ));
    assert!(matches!(
        request_with_question(json!({"type": "score", "criteria": []}))
            .expect("the request must parse")
            .translate(),
        Err(TypeSafeTranslationError::ScoreCriteriaOutOfRange {
            criteria_count,
            ..
        }) if criteria_count == 0
    ));
}
