use std::time::UNIX_EPOCH;

use serde_json::from_value;
use serde_json::json;

use paddler_messaging::request_params_validation_error::RequestParamsValidationError;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_openai_translation::openai_translation_error::OpenAITranslationError;

#[test]
fn a_chat_completion_request_with_an_unsatisfiable_tool_is_refused() {
    let chat_completion_request: ChatCompletionRequest = from_value(json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "hi"}],
        "tools": [{
            "type": "function",
            "function": {
                "name": "broken",
                "description": "tool with an unsatisfiable required field",
                "parameters": {
                    "type": "object",
                    "properties": {"present": {"type": "string"}},
                    "required": ["absent"]
                }
            }
        }]
    }))
    .expect("the request must deserialize");

    assert_eq!(
        chat_completion_request
            .translate(UNIX_EPOCH)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            OpenAITranslationError::ToolRejected(
                RequestParamsValidationError::RequiredFieldNotInProperties {
                    field: "absent".to_owned(),
                }
            )
            .to_string()
        )
    );
}
