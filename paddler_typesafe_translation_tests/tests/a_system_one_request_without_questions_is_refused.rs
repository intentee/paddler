use serde_json::json;

use paddler_messaging::request_params_validation_error::RequestParamsValidationError;
use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;
use paddler_typesafe_translation_tests::system_one_request_with_questions::system_one_request_with_questions;

#[test]
fn a_system_one_request_without_questions_is_refused() {
    let request = system_one_request_with_questions(&json!({})).expect("the request must parse");

    assert!(matches!(
        request.translate(),
        Err(TypeSafeTranslationError::DecisionInvalid(
            RequestParamsValidationError::DecisionWithoutQuestions
        ))
    ));
}
