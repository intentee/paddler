use actix_web::HttpResponse;
use actix_web::http::StatusCode;

use paddler_openai_translation::generation_failure::GenerationFailure;
use paddler_openai_translation::generation_failure_cause::GenerationFailureCause;
use paddler_openai_translation::openai_translation_error::OpenAITranslationError;

use crate::compatibility::openai_service::openai_error_body::OpenAIErrorBody;
use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;
use crate::compatibility::upstream_failure::UpstreamFailure;
use crate::relay_failure::RelayFailure;

pub enum OpenAIFailure {
    Generation(GenerationFailure),
    Relay(RelayFailure),
    RequestUntranslatable(OpenAITranslationError),
}

impl OpenAIFailure {
    #[must_use]
    pub fn into_error_body(self) -> OpenAIErrorBody {
        let status_code = self.status_code();

        self.into_error_body_for(status_code)
    }

    #[must_use]
    pub fn into_http_response(self) -> HttpResponse {
        let status_code = self.status_code();

        self.into_error_body_for(status_code)
            .into_http_response(status_code)
    }

    fn into_error_body_for(self, status_code: StatusCode) -> OpenAIErrorBody {
        OpenAIErrorBody {
            error_type: OpenAIErrorType::from(status_code),
            message: self.into_message(),
        }
    }

    fn into_message(self) -> String {
        match self {
            Self::Generation(GenerationFailure { message, .. }) => message,
            Self::Relay(relay_failure) => relay_failure.description(),
            Self::RequestUntranslatable(translation_error) => translation_error.to_string(),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Generation(GenerationFailure {
                cause: GenerationFailureCause::InvalidRequest,
                ..
            })
            | Self::RequestUntranslatable(
                OpenAITranslationError::MessageWithoutContent { .. }
                | OpenAITranslationError::ReasoningItemWithoutContent
                | OpenAITranslationError::ReasoningOnNonAssistantMessage { .. }
                | OpenAITranslationError::ToolCallArgumentsAreNotAJsonObject { .. }
                | OpenAITranslationError::ToolCallsOnNonAssistantMessage { .. }
                | OpenAITranslationError::ToolRejected(_)
                | OpenAITranslationError::ToolResultWithoutToolCallId,
            ) => StatusCode::BAD_REQUEST,
            Self::Generation(GenerationFailure {
                cause: GenerationFailureCause::Unavailable,
                ..
            }) => UpstreamFailure::Unavailable.status_code(),
            Self::Generation(GenerationFailure {
                cause: GenerationFailureCause::AgentFailed,
                ..
            }) => UpstreamFailure::AgentFailed.status_code(),
            Self::RequestUntranslatable(OpenAITranslationError::ClockBeforeUnixEpoch(_)) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Relay(relay_failure) => UpstreamFailure::from(relay_failure).status_code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;
    use serde_json::Map;
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::from_str;
    use serde_json::json;

    use paddler_messaging::request_params_validation_error::RequestParamsValidationError;
    use paddler_openai_translation::chat_completion_message_role::ChatCompletionMessageRole;
    use paddler_openai_translation::generation_failure::GenerationFailure;
    use paddler_openai_translation::generation_failure_cause::GenerationFailureCause;
    use paddler_openai_translation::openai_translation_error::OpenAITranslationError;

    use super::OpenAIFailure;
    use crate::relay_failure::RelayFailure;

    fn error_body(message: &str, error_type: &str) -> Value {
        json!({
            "error": {
                "message": message,
                "type": error_type,
                "param": null,
                "code": null
            }
        })
    }

    fn generation_failure(cause: GenerationFailureCause, message: &str) -> OpenAIFailure {
        OpenAIFailure::Generation(GenerationFailure {
            cause,
            message: message.to_owned(),
        })
    }

    #[actix_web::test]
    async fn answers_every_failure_with_its_status_and_openai_error_body() {
        let clock_error = (UNIX_EPOCH - Duration::from_secs(1))
            .duration_since(UNIX_EPOCH)
            .unwrap_err();

        for (openai_failure, expected_status, expected_body) in [
            (
                generation_failure(GenerationFailureCause::InvalidRequest, "client mistake"),
                StatusCode::BAD_REQUEST,
                error_body("client mistake", "invalid_request_error"),
            ),
            (
                generation_failure(GenerationFailureCause::Unavailable, "no model is loaded"),
                StatusCode::SERVICE_UNAVAILABLE,
                error_body("no model is loaded", "server_error"),
            ),
            (
                generation_failure(GenerationFailureCause::AgentFailed, "sampler blew up"),
                StatusCode::INTERNAL_SERVER_ERROR,
                error_body("sampler blew up", "server_error"),
            ),
            (
                OpenAIFailure::RequestUntranslatable(OpenAITranslationError::ToolRejected(
                    RequestParamsValidationError::RequiredFieldNotInProperties {
                        field: "absent".to_owned(),
                    },
                )),
                StatusCode::BAD_REQUEST,
                error_body(
                    "Required field 'absent' not found in properties",
                    "invalid_request_error",
                ),
            ),
            (
                OpenAIFailure::RequestUntranslatable(
                    OpenAITranslationError::MessageWithoutContent {
                        role: ChatCompletionMessageRole::User,
                    },
                ),
                StatusCode::BAD_REQUEST,
                error_body(
                    "a 'user' message must have content",
                    "invalid_request_error",
                ),
            ),
            (
                OpenAIFailure::RequestUntranslatable(
                    OpenAITranslationError::ReasoningOnNonAssistantMessage {
                        role: ChatCompletionMessageRole::Developer,
                    },
                ),
                StatusCode::BAD_REQUEST,
                error_body(
                    "only an assistant message can carry reasoning, not a 'developer' message",
                    "invalid_request_error",
                ),
            ),
            (
                OpenAIFailure::RequestUntranslatable(
                    OpenAITranslationError::ReasoningItemWithoutContent,
                ),
                StatusCode::BAD_REQUEST,
                error_body(
                    "a reasoning item must carry its reasoning text",
                    "invalid_request_error",
                ),
            ),
            (
                OpenAIFailure::RequestUntranslatable(
                    OpenAITranslationError::ToolCallArgumentsAreNotAJsonObject {
                        tool_call_id: "call_0".to_owned(),
                        source: from_str::<Map<String, Value>>("[]").unwrap_err(),
                    },
                ),
                StatusCode::BAD_REQUEST,
                error_body(
                    "the arguments of the tool call 'call_0' are not a JSON object",
                    "invalid_request_error",
                ),
            ),
            (
                OpenAIFailure::RequestUntranslatable(
                    OpenAITranslationError::ToolCallsOnNonAssistantMessage {
                        role: ChatCompletionMessageRole::User,
                    },
                ),
                StatusCode::BAD_REQUEST,
                error_body(
                    "only an assistant message can request tool calls, not a 'user' message",
                    "invalid_request_error",
                ),
            ),
            (
                OpenAIFailure::RequestUntranslatable(
                    OpenAITranslationError::ToolResultWithoutToolCallId,
                ),
                StatusCode::BAD_REQUEST,
                error_body(
                    "a tool message must name the tool call it answers in 'tool_call_id'",
                    "invalid_request_error",
                ),
            ),
            (
                OpenAIFailure::RequestUntranslatable(OpenAITranslationError::ClockBeforeUnixEpoch(
                    clock_error,
                )),
                StatusCode::INTERNAL_SERVER_ERROR,
                error_body(
                    "the system clock reads a time before the Unix epoch",
                    "server_error",
                ),
            ),
            (
                OpenAIFailure::Relay(RelayFailure::BufferedRequestTimedOut),
                StatusCode::GATEWAY_TIMEOUT,
                error_body("Waiting for available slot timed out", "server_error"),
            ),
        ] {
            let response = openai_failure.into_http_response();

            assert_eq!(response.status(), expected_status);
            assert_eq!(
                from_slice::<Value>(&to_bytes(response.into_body()).await.unwrap()).unwrap(),
                expected_body
            );
        }
    }
}
