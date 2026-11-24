use actix_web::HttpResponse;
use actix_web::http::StatusCode;

use paddler_balancer::compatibility::upstream_failure::UpstreamFailure;
use paddler_balancer::relay_failure::RelayFailure;
use paddler_openai_translation::generation_failure::GenerationFailure;
use paddler_openai_translation::generation_failure_cause::GenerationFailureCause;
use paddler_openai_translation::openai_translation_error::OpenAITranslationError;

use crate::openai_error_body::OpenAIErrorBody;
use crate::openai_error_type::OpenAIErrorType;

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

    use actix_web::http::StatusCode;
    use serde_json::Map;
    use serde_json::Value;
    use serde_json::from_str;

    use paddler_balancer::relay_failure::RelayFailure;
    use paddler_messaging::request_params_validation_error::RequestParamsValidationError;
    use paddler_openai_translation::chat_completion_message_role::ChatCompletionMessageRole;
    use paddler_openai_translation::generation_failure::GenerationFailure;
    use paddler_openai_translation::generation_failure_cause::GenerationFailureCause;
    use paddler_openai_translation::openai_translation_error::OpenAITranslationError;

    use super::OpenAIFailure;
    use crate::openai_error_type::OpenAIErrorType;

    fn assert_answered_with(
        openai_failure: OpenAIFailure,
        expected_status: StatusCode,
        expected_error_type: OpenAIErrorType,
    ) {
        assert_eq!(openai_failure.status_code(), expected_status);
        assert_eq!(
            openai_failure.into_error_body().error_type,
            expected_error_type
        );
    }

    fn generation_failure(cause: GenerationFailureCause) -> OpenAIFailure {
        OpenAIFailure::Generation(GenerationFailure {
            cause,
            message: "generation failed".to_owned(),
        })
    }

    #[test]
    fn answers_a_generation_failure_with_the_status_of_its_cause() {
        assert_answered_with(
            generation_failure(GenerationFailureCause::InvalidRequest),
            StatusCode::BAD_REQUEST,
            OpenAIErrorType::InvalidRequestError,
        );
        assert_answered_with(
            generation_failure(GenerationFailureCause::Unavailable),
            StatusCode::SERVICE_UNAVAILABLE,
            OpenAIErrorType::ServerError,
        );
        assert_answered_with(
            generation_failure(GenerationFailureCause::AgentFailed),
            StatusCode::INTERNAL_SERVER_ERROR,
            OpenAIErrorType::ServerError,
        );
    }

    #[test]
    fn answers_a_request_the_client_must_correct_as_an_invalid_request() {
        for translation_error in [
            OpenAITranslationError::ToolRejected(
                RequestParamsValidationError::RequiredFieldNotInProperties {
                    field: "absent".to_owned(),
                },
            ),
            OpenAITranslationError::MessageWithoutContent {
                role: ChatCompletionMessageRole::User,
            },
            OpenAITranslationError::ReasoningOnNonAssistantMessage {
                role: ChatCompletionMessageRole::Developer,
            },
            OpenAITranslationError::ReasoningItemWithoutContent,
            OpenAITranslationError::ToolCallArgumentsAreNotAJsonObject {
                tool_call_id: "call_0".to_owned(),
                source: from_str::<Map<String, Value>>("[]")
                    .expect_err("a JSON array must not parse as a JSON object"),
            },
            OpenAITranslationError::ToolCallsOnNonAssistantMessage {
                role: ChatCompletionMessageRole::User,
            },
            OpenAITranslationError::ToolResultWithoutToolCallId,
        ] {
            assert_answered_with(
                OpenAIFailure::RequestUntranslatable(translation_error),
                StatusCode::BAD_REQUEST,
                OpenAIErrorType::InvalidRequestError,
            );
        }
    }

    #[test]
    fn answers_a_clock_before_the_unix_epoch_as_a_server_error() {
        assert_answered_with(
            OpenAIFailure::RequestUntranslatable(OpenAITranslationError::ClockBeforeUnixEpoch(
                UNIX_EPOCH
                    .checked_sub(Duration::from_secs(1))
                    .expect("one second before the Unix epoch must be representable")
                    .duration_since(UNIX_EPOCH)
                    .expect_err("a time before the Unix epoch must have no duration since it"),
            )),
            StatusCode::INTERNAL_SERVER_ERROR,
            OpenAIErrorType::ServerError,
        );
    }

    #[test]
    fn answers_a_relay_failure_with_the_status_of_its_upstream_failure() {
        assert_answered_with(
            OpenAIFailure::Relay(RelayFailure::BufferedRequestTimedOut),
            StatusCode::GATEWAY_TIMEOUT,
            OpenAIErrorType::ServerError,
        );
    }
}
