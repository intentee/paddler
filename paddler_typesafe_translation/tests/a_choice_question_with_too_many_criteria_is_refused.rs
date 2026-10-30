use serde_json::Map;
use serde_json::Value;
use serde_json::json;

use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

use crate::system_one_request_with_question::system_one_request_with_question;

const TOO_MANY_CRITERIA: usize = 256;

#[test]
fn a_choice_question_with_too_many_criteria_is_refused() {
    let too_many_choices: Map<String, Value> = (0..TOO_MANY_CRITERIA)
        .map(|option| (format!("option-{option}"), Value::Null))
        .collect();

    assert_eq!(
        system_one_request_with_question(json!({"type": "choice", "criteria": too_many_choices}))
            .expect("the request must parse")
            .translate()
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TypeSafeTranslationError::ChoiceCriteriaOutOfRange {
                criteria_count: TOO_MANY_CRITERIA,
                question_id: "question".to_owned(),
            }
            .to_string()
        )
    );
}
