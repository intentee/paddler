use actix_web::HttpResponse;
use actix_web::http::StatusCode;

use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_openai_translation::generation_failure::GenerationFailure;
use paddler_openai_translation::generation_failure_cause::GenerationFailureCause;
use paddler_openai_translation::openai_translation_error::OpenAITranslationError;

use crate::compatibility::openai_service::openai_error_body::OpenAIErrorBody;
use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;
use crate::compatibility::upstream_failure::UpstreamFailure;

pub enum OpenAIFailure {
    Generation(GenerationFailure),
    RequestUntranslatable(OpenAITranslationError),
    Unfinished,
    Wire(JsonRpcError),
}

impl OpenAIFailure {
    #[must_use]
    pub fn into_error_body(self) -> OpenAIErrorBody {
        let error_type = if self.status_code().is_client_error() {
            OpenAIErrorType::InvalidRequestError
        } else {
            OpenAIErrorType::ServerError
        };

        OpenAIErrorBody {
            error_type,
            message: self.into_message(),
        }
    }

    #[must_use]
    pub fn into_http_response(self) -> HttpResponse {
        let status_code = self.status_code();

        self.into_error_body().into_http_response(status_code)
    }

    fn into_message(self) -> String {
        match self {
            Self::Generation(GenerationFailure { message, .. }) => message,
            Self::RequestUntranslatable(translation_error) => translation_error.to_string(),
            Self::Unfinished => "the agent stopped before it finished the generation".to_owned(),
            Self::Wire(JsonRpcError { description, .. }) => description,
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Generation(GenerationFailure {
                cause: GenerationFailureCause::InvalidRequest,
                ..
            })
            | Self::RequestUntranslatable(OpenAITranslationError::ToolRejected(_)) => {
                StatusCode::BAD_REQUEST
            }
            Self::Generation(GenerationFailure {
                cause: GenerationFailureCause::Unavailable,
                ..
            })
            | Self::RequestUntranslatable(OpenAITranslationError::InferenceModeMismatch {
                ..
            }) => UpstreamFailure::Unavailable.status_code(),
            Self::Generation(GenerationFailure {
                cause: GenerationFailureCause::AgentFailed,
                ..
            }) => UpstreamFailure::AgentFailed.status_code(),
            Self::RequestUntranslatable(OpenAITranslationError::ClockBeforeUnixEpoch(_)) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Unfinished => UpstreamFailure::RelayFailed.status_code(),
            Self::Wire(wire_error) => UpstreamFailure::from(wire_error).status_code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::json;

    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::request_params_validation_error::RequestParamsValidationError;
    use paddler_openai_translation::generation_failure::GenerationFailure;
    use paddler_openai_translation::generation_failure_cause::GenerationFailureCause;
    use paddler_openai_translation::openai_translation_error::OpenAITranslationError;

    use super::OpenAIFailure;

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
                    OpenAITranslationError::InferenceModeMismatch {
                        inference_mode: InferenceMode::Embeddings,
                    },
                ),
                StatusCode::SERVICE_UNAVAILABLE,
                error_body(
                    "the cluster serves Embeddings, not text generation",
                    "server_error",
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
                OpenAIFailure::Unfinished,
                StatusCode::BAD_GATEWAY,
                error_body(
                    "the agent stopped before it finished the generation",
                    "server_error",
                ),
            ),
            (
                OpenAIFailure::Wire(JsonRpcError {
                    code: 504,
                    description: "timed out".to_owned(),
                }),
                StatusCode::GATEWAY_TIMEOUT,
                error_body("timed out", "server_error"),
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
