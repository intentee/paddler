use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use async_trait::async_trait;
use nanoid::nanoid;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::inference_client::Message as OutgoingMessage;
use paddler_types::inference_client::Response as OutgoingResponse;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::jsonrpc::ResponseEnvelope;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::StreamExt as _;

use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer::compatibility::openai_service::app_data::AppData;
use crate::balancer::http_stream_from_agent::http_stream_from_agent;
use crate::balancer::unbounded_stream_from_agent::unbounded_stream_from_agent;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

fn openai_error_json(error_type: &str, message: &str) -> serde_json::Value {
    json!({
        "error": {
            "message": message,
            "type": error_type,
            "param": null,
            "code": null
        }
    })
}

#[expect(
    clippy::expect_used,
    reason = "system time before UNIX_EPOCH means we are moving back in time"
)]
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs()
}

#[derive(Deserialize)]
/// Although fields are same as in Paddler's conversation message for the moment,
/// it would be better if this struct stayed independent from ours just in case
/// to avoid any potential side effects in the future.
struct OpenAIMessage {
    content: ConversationMessageContent,
    role: String,
}

impl From<&OpenAIMessage> for ConversationMessage {
    fn from(openai_message: &OpenAIMessage) -> Self {
        Self {
            content: openai_message.content.clone(),
            role: openai_message.role.clone(),
        }
    }
}

#[derive(Deserialize)]
struct OpenAICompletionRequestParams {
    max_completion_tokens: Option<i32>,
    messages: Vec<OpenAIMessage>,
    /// This parameter is ignored here, but is required by the `OpenAI` API.
    model: String,
    stream: Option<bool>,
}

#[derive(Clone)]
struct OpenAIStreamingResponseTransformer {
    model: String,
    system_fingerprint: String,
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAIStreamingResponseTransformer {
    type TransformedMessage = serde_json::Value;

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> anyhow::Result<Self::TransformedMessage> {
        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done),
            }) => Ok(json!({
                "id": request_id,
                "object": "chat.completion.chunk",
                "created": current_timestamp(),
                "model": self.model,
                "system_fingerprint": self.system_fingerprint,
                "choices": [
                    {
                        "index": 0,
                        "delta": {},
                        "logprobs": null,
                        "finish_reason": "stop"
                    }
                ]
            })),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Token(token)),
            }) => Ok(json!({
                "id": request_id,
                "object": "chat.completion.chunk",
                "created": current_timestamp(),
                "model": self.model,
                "system_fingerprint": self.system_fingerprint,
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "role": "assistant",
                            "content": token,
                        },
                        "logprobs": null,
                        "finish_reason": null
                    }
                ]
            })),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ChatTemplateError(description)
                        | GeneratedTokenResult::ImageDecodingFailed(description)
                        | GeneratedTokenResult::MultimodalNotSupported(description),
                    ),
                ..
            })
            | OutgoingMessage::Error(ErrorEnvelope {
                error: paddler_types::jsonrpc::Error { description, .. },
                ..
            }) => Ok(openai_error_json("server_error", &description)),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Timeout,
                ..
            }) => Ok(openai_error_json("timeout", "request timed out")),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::TooManyBufferedRequests,
                ..
            }) => Ok(openai_error_json(
                "rate_limit_error",
                "too many buffered requests",
            )),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Embedding(_),
                ..
            }) => Ok(openai_error_json(
                "invalid_request_error",
                "unexpected embedding response in chat completions",
            )),
        }
    }
}

#[derive(Clone)]
struct OpenAICombinedResponseTransformer {}

#[async_trait]
impl TransformsOutgoingMessage for OpenAICombinedResponseTransformer {
    type TransformedMessage = String;

    fn stringify(&self, message: &Self::TransformedMessage) -> anyhow::Result<String> {
        Ok(message.clone())
    }

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> anyhow::Result<Self::TransformedMessage> {
        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done),
                ..
            }) => Ok(String::new()),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Token(token)),
                ..
            }) => Ok(token),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ChatTemplateError(description)
                        | GeneratedTokenResult::ImageDecodingFailed(description)
                        | GeneratedTokenResult::MultimodalNotSupported(description),
                    ),
                ..
            })
            | OutgoingMessage::Error(ErrorEnvelope {
                error: paddler_types::jsonrpc::Error { description, .. },
                ..
            }) => Ok(openai_error_json("server_error", &description).to_string()),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Timeout,
                ..
            }) => Ok(openai_error_json("timeout", "request timed out").to_string()),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::TooManyBufferedRequests,
                ..
            }) => {
                Ok(openai_error_json("rate_limit_error", "too many buffered requests").to_string())
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Embedding(_),
                ..
            }) => Ok(openai_error_json(
                "invalid_request_error",
                "unexpected embedding response in chat completions",
            )
            .to_string()),
        }
    }
}

#[post("/v1/chat/completions")]
async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAICompletionRequestParams>,
) -> Result<HttpResponse, Error> {
    let paddler_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(
            openai_params
                .messages
                .iter()
                .map(ConversationMessage::from)
                .collect(),
        ),
        enable_thinking: true,
        max_tokens: openai_params.max_completion_tokens.unwrap_or(2000),
        tools: vec![],
    };

    if openai_params.stream.unwrap_or(false) {
        Ok(http_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAIStreamingResponseTransformer {
                model: openai_params.model.clone(),
                system_fingerprint: nanoid!(),
            },
        ))
    } else {
        let combined_response = unbounded_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAICombinedResponseTransformer {},
        )
        .collect::<Vec<String>>()
        .await
        .join("");

        Ok(HttpResponse::Ok().json(json!({
          "id": nanoid!(),
          "object": "chat.completion",
          "created": current_timestamp(),
          "model": openai_params.model,
          "choices": [
            {
              "index": 0,
              "message": {
                "role": "assistant",
                "content": combined_response,
                "refusal": null,
                "annotations": []
              },
              "logprobs": null,
              "finish_reason": "stop"
            }
          ],
          "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
            "prompt_tokens_details": {
              "cached_tokens": 0,
              "audio_tokens": 0
            },
            "completion_tokens_details": {
              "reasoning_tokens": 0,
              "audio_tokens": 0,
              "accepted_prediction_tokens": 0,
              "rejected_prediction_tokens": 0
            }
          },
          "service_tier": "default"
        })))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use paddler_types::generated_token_result::GeneratedTokenResult;
    use paddler_types::inference_client::Message as OutgoingMessage;
    use paddler_types::inference_client::Response as OutgoingResponse;
    use paddler_types::jsonrpc::ErrorEnvelope;
    use paddler_types::jsonrpc::ResponseEnvelope;

    use super::OpenAICombinedResponseTransformer;
    use super::OpenAIStreamingResponseTransformer;
    use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;

    fn make_token_message(token_result: GeneratedTokenResult) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::GeneratedToken(token_result),
        })
    }

    fn make_error_message(code: i32, description: &str) -> OutgoingMessage {
        OutgoingMessage::Error(ErrorEnvelope {
            request_id: "test-request".to_owned(),
            error: paddler_types::jsonrpc::Error {
                code,
                description: description.to_owned(),
            },
        })
    }

    fn make_response_message(response: OutgoingResponse) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            request_id: "test-request".to_owned(),
            response,
        })
    }

    #[actix_web::test]
    async fn streaming_token_emits_content_delta() -> Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_owned(),
            system_fingerprint: "test-fingerprint".to_owned(),
        };

        let message = make_token_message(GeneratedTokenResult::Token("hello".to_owned()));
        let result = transformer.transform(message).await?;

        assert_eq!(result["choices"][0]["delta"]["content"], "hello");
        assert_eq!(result["choices"][0]["delta"]["role"], "assistant");
        assert!(result["choices"][0]["finish_reason"].is_null());

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_done_emits_stop_finish_reason() -> Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_owned(),
            system_fingerprint: "test-fingerprint".to_owned(),
        };

        let message = make_token_message(GeneratedTokenResult::Done);
        let result = transformer.transform(message).await?;

        assert_eq!(result["choices"][0]["finish_reason"], "stop");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_token_returns_content() -> Result<()> {
        let transformer = OpenAICombinedResponseTransformer {};

        let message = make_token_message(GeneratedTokenResult::Token("hello".to_owned()));
        let result = transformer.transform(message).await?;

        assert_eq!(result, "hello");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_done_returns_empty_string() -> Result<()> {
        let transformer = OpenAICombinedResponseTransformer {};

        let message = make_token_message(GeneratedTokenResult::Done);
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_error_message_returns_openai_error() -> Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_owned(),
            system_fingerprint: "test-fingerprint".to_owned(),
        };

        let message = make_error_message(500, "internal server error");
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["type"], "server_error");
        assert_eq!(result["error"]["message"], "internal server error");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_error_message_returns_openai_error() -> Result<()> {
        let transformer = OpenAICombinedResponseTransformer {};

        let message = make_error_message(500, "internal server error");
        let result = transformer.transform(message).await?;

        assert!(result.contains("internal server error"));
        assert!(result.contains("server_error"));

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_chat_template_error_returns_openai_error() -> Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_owned(),
            system_fingerprint: "test-fingerprint".to_owned(),
        };

        let message = make_token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_owned(),
        ));
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["type"], "server_error");
        assert_eq!(result["error"]["message"], "bad template");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_chat_template_error_returns_openai_error() -> Result<()> {
        let transformer = OpenAICombinedResponseTransformer {};

        let message = make_token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_owned(),
        ));
        let result = transformer.transform(message).await?;

        assert!(result.contains("bad template"));
        assert!(result.contains("server_error"));

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_timeout_returns_openai_error() -> Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_owned(),
            system_fingerprint: "test-fingerprint".to_owned(),
        };

        let message = make_response_message(OutgoingResponse::Timeout);
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["type"], "timeout");
        assert_eq!(result["error"]["message"], "request timed out");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_timeout_returns_openai_error() -> Result<()> {
        let transformer = OpenAICombinedResponseTransformer {};

        let message = make_response_message(OutgoingResponse::Timeout);
        let result = transformer.transform(message).await?;

        assert!(result.contains("request timed out"));
        assert!(result.contains("timeout"));

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_too_many_buffered_requests_returns_openai_error() -> Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_owned(),
            system_fingerprint: "test-fingerprint".to_owned(),
        };

        let message = make_response_message(OutgoingResponse::TooManyBufferedRequests);
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["type"], "rate_limit_error");
        assert_eq!(result["error"]["message"], "too many buffered requests");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_too_many_buffered_requests_returns_openai_error() -> Result<()> {
        let transformer = OpenAICombinedResponseTransformer {};

        let message = make_response_message(OutgoingResponse::TooManyBufferedRequests);
        let result = transformer.transform(message).await?;

        assert!(result.contains("too many buffered requests"));
        assert!(result.contains("rate_limit_error"));

        Ok(())
    }
}
