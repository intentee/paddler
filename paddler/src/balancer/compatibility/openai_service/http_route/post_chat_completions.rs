use std::sync::Arc;
use std::sync::Mutex;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use anyhow::anyhow;
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
use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::FunctionCall;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_types::validates::Validates;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::StreamExt as _;

use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer::compatibility::openai_service::app_data::AppData;
use crate::balancer::compatibility::openai_service::http_route::current_timestamp;
use crate::balancer::compatibility::openai_service::http_route::openai_http_stream_from_agent::openai_http_stream_from_agent;
use crate::balancer::unbounded_stream_from_agent::unbounded_stream_from_agent;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[derive(Deserialize)]
struct OpenAIFunction {
    name: String,
    description: Option<String>,
    parameters: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct OpenAITool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunction,
}

impl TryFrom<&OpenAITool> for Tool<ValidatedParametersSchema> {
    type Error = anyhow::Error;

    fn try_from(openai_tool: &OpenAITool) -> anyhow::Result<Self> {
        if openai_tool.tool_type != "function" {
            return Err(anyhow!("unsupported tool type: {}", openai_tool.tool_type));
        }

        let parameters = match &openai_tool.function.parameters {
            None => Parameters::Empty,
            Some(value) => {
                let filtered = match value {
                    serde_json::Value::Object(map) => {
                        let known_keys = ["type", "properties", "required", "additionalProperties"];
                        serde_json::Value::Object(
                            map.iter()
                                .filter(|(key, _)| known_keys.contains(&key.as_str()))
                                .map(|(key, value)| (key.clone(), value.clone()))
                                .collect(),
                        )
                    }
                    other => other.clone(),
                };
                let raw_schema: RawParametersSchema = serde_json::from_value(filtered)?;

                Parameters::Schema(raw_schema.validate()?)
            }
        };

        Ok(Tool::Function(FunctionCall {
            function: Function {
                name: openai_tool.function.name.clone(),
                description: openai_tool.function.description.clone().unwrap_or_default(),
                parameters,
            },
        }))
    }
}

#[derive(Deserialize)]
/// Although fields are same as in Paddler's conversation message for the moment,
/// it would be better if this struct stayed independent from ours just in case
/// to avoid any potential side effects in the future.
struct OpenAIMessage {
    content: Option<ConversationMessageContent>,
    role: String,
    // accepted for OpenAI protocol compatibility, not forwarded to conversation history
    name: Option<String>,
    // accepted for OpenAI protocol compatibility, not forwarded to conversation history
    tool_call_id: Option<String>,
    // accepted for OpenAI protocol compatibility, not forwarded to conversation history
    tool_calls: Option<Vec<serde_json::Value>>,
}

impl From<&OpenAIMessage> for ConversationMessage {
    fn from(openai_message: &OpenAIMessage) -> Self {
        ConversationMessage {
            content: openai_message
                .content
                .clone()
                .unwrap_or(ConversationMessageContent::Text(String::new())),
            role: openai_message.role.clone(),
        }
    }
}

#[derive(Deserialize)]
struct OpenAICompletionRequestParams {
    // accepted for OpenAI protocol compatibility, not forwarded
    frequency_penalty: Option<f32>,
    logprobs: Option<bool>,
    max_completion_tokens: Option<i32>,
    messages: Vec<OpenAIMessage>,
    /// This parameter is ignored here, but is required by the OpenAI API.
    model: String,
    n: Option<u32>,
    // accepted for OpenAI protocol compatibility, not forwarded
    presence_penalty: Option<f32>,
    response_format: Option<serde_json::Value>,
    seed: Option<i64>,
    stop: Option<serde_json::Value>,
    stream: Option<bool>,
    // accepted for OpenAI protocol compatibility, not forwarded
    temperature: Option<f32>,
    tool_choice: Option<serde_json::Value>,
    tools: Option<Vec<OpenAITool>>,
    top_logprobs: Option<u32>,
    // accepted for OpenAI protocol compatibility, not forwarded
    top_p: Option<f32>,
    user: Option<String>,
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
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ChatTemplateError(error)),
                ..
            }) => Ok(
                json!({"error":{"message":format!("chat template error: {error}"),"type":"server_error","param":serde_json::Value::Null,"code":"chat_template_error"}}),
            ),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ImageDecodingFailed(error)),
                ..
            }) => Ok(
                json!({"error":{"message":format!("image decoding failed: {error}"),"type":"server_error","param":serde_json::Value::Null,"code":"image_decoding_failed"}}),
            ),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::MultimodalNotSupported(
                        error,
                    )),
                ..
            }) => Ok(
                json!({"error":{"message":format!("multimodal not supported: {error}"),"type":"server_error","param":serde_json::Value::Null,"code":"multimodal_not_supported"}}),
            ),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Timeout,
                ..
            }) => Ok(
                json!({"error":{"message":"inference timeout","type":"server_error","param":serde_json::Value::Null,"code":"timeout"}}),
            ),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::TooManyBufferedRequests,
                ..
            }) => Ok(
                json!({"error":{"message":"too many buffered requests","type":"server_error","param":serde_json::Value::Null,"code":"too_many_buffered_requests"}}),
            ),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Embedding(_),
                ..
            }) => Ok(
                json!({"error":{"message":"unexpected embedding response","type":"server_error","param":serde_json::Value::Null,"code":"unexpected_embedding_response"}}),
            ),
            OutgoingMessage::Error(ErrorEnvelope { error, .. }) => Ok(
                json!({"error":{"message":format!("inference error: {}", error.description),"type":"server_error","param":serde_json::Value::Null,"code":"inference_error"}}),
            ),
        }
    }
}

#[derive(Clone)]
struct OpenAICombinedResponseTransformer {
    captured_error: Arc<Mutex<Option<serde_json::Value>>>,
}

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
            }) => Ok("".to_string()),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Token(token)),
                ..
            }) => Ok(token),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ChatTemplateError(error)),
                ..
            }) => {
                *self.captured_error.lock().expect("Poisoned error lock") = Some(json!({
                    "error": {
                        "message": format!("chat template error: {error}"),
                        "type": "server_error",
                        "param": serde_json::Value::Null,
                        "code": "chat_template_error"
                    }
                }));

                Ok("".to_string())
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ImageDecodingFailed(error)),
                ..
            }) => {
                *self.captured_error.lock().expect("Poisoned error lock") = Some(json!({
                    "error": {
                        "message": format!("image decoding failed: {error}"),
                        "type": "server_error",
                        "param": serde_json::Value::Null,
                        "code": "image_decoding_failed"
                    }
                }));

                Ok("".to_string())
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::MultimodalNotSupported(
                        error,
                    )),
                ..
            }) => {
                *self.captured_error.lock().expect("Poisoned error lock") = Some(json!({
                    "error": {
                        "message": format!("multimodal not supported: {error}"),
                        "type": "server_error",
                        "param": serde_json::Value::Null,
                        "code": "multimodal_not_supported"
                    }
                }));

                Ok("".to_string())
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Timeout,
                ..
            }) => {
                *self.captured_error.lock().expect("Poisoned error lock") = Some(json!({
                    "error": {
                        "message": "inference timeout",
                        "type": "server_error",
                        "param": serde_json::Value::Null,
                        "code": "timeout"
                    }
                }));

                Ok("".to_string())
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::TooManyBufferedRequests,
                ..
            }) => {
                *self.captured_error.lock().expect("Poisoned error lock") = Some(json!({
                    "error": {
                        "message": "too many buffered requests",
                        "type": "server_error",
                        "param": serde_json::Value::Null,
                        "code": "too_many_buffered_requests"
                    }
                }));

                Ok("".to_string())
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Embedding(_),
                ..
            }) => {
                *self.captured_error.lock().expect("Poisoned error lock") = Some(json!({
                    "error": {
                        "message": "unexpected embedding response",
                        "type": "server_error",
                        "param": serde_json::Value::Null,
                        "code": "unexpected_embedding_response"
                    }
                }));

                Ok("".to_string())
            }
            OutgoingMessage::Error(ErrorEnvelope { error, .. }) => {
                *self.captured_error.lock().expect("Poisoned error lock") = Some(json!({
                    "error": {
                        "message": format!("inference error: {}", error.description),
                        "type": "server_error",
                        "param": serde_json::Value::Null,
                        "code": "inference_error"
                    }
                }));

                Ok("".to_string())
            }
        }
    }
}

#[post("/v1/chat/completions")]
async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAICompletionRequestParams>,
) -> Result<HttpResponse, Error> {
    if openai_params.n.unwrap_or(1) != 1 {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": {
                "message": "n parameter must be 1; multiple completions are not supported",
                "type": "invalid_request_error",
                "param": "n",
                "code": "unsupported_value"
            }
        })));
    }

    if let Some(max_tokens) = openai_params.max_completion_tokens {
        if max_tokens <= 0 {
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": {
                    "message": "max_completion_tokens must be greater than 0",
                    "type": "invalid_request_error",
                    "param": "max_completion_tokens",
                    "code": "invalid_value"
                }
            })));
        }
    }

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
        tools: {
            let tool_result: anyhow::Result<Vec<_>> = openai_params
                .tools
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(Tool::try_from)
                .collect();
            match tool_result {
                Ok(tools) => tools,
                Err(err) => {
                    return Ok(HttpResponse::BadRequest().json(json!({
                        "error": {
                            "message": err.to_string(),
                            "type": "invalid_request_error",
                            "param": "tools",
                            "code": "invalid_tool"
                        }
                    })));
                }
            }
        },
    };

    if openai_params.stream.unwrap_or(false) {
        openai_http_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAIStreamingResponseTransformer {
                model: openai_params.model.clone(),
                system_fingerprint: nanoid!(),
            },
        )
    } else {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };
        let captured_error = transformer.captured_error.clone();

        let combined_response = unbounded_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            transformer,
        )?
        .fold(String::new(), |mut accumulated, chunk| {
            accumulated.push_str(&chunk);
            accumulated
        })
        .await;

        if let Some(error_json) = captured_error.lock().expect("Poisoned error lock").take() {
            let status = match error_json["error"]["code"].as_str() {
                Some("timeout") => actix_web::http::StatusCode::GATEWAY_TIMEOUT,
                Some("too_many_buffered_requests") => {
                    actix_web::http::StatusCode::TOO_MANY_REQUESTS
                }
                _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            };

            return Ok(HttpResponse::build(status).json(error_json));
        }

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
    use std::sync::Arc;
    use std::sync::Mutex;

    use paddler_types::embedding::Embedding;
    use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_types::embedding_result::EmbeddingResult;
    use paddler_types::generated_token_result::GeneratedTokenResult;
    use paddler_types::inference_client::Message as OutgoingMessage;
    use paddler_types::inference_client::Response as OutgoingResponse;
    use paddler_types::jsonrpc::Error as JsonRpcError;
    use paddler_types::jsonrpc::ErrorEnvelope;
    use paddler_types::jsonrpc::ResponseEnvelope;
    use paddler_types::pooling_type::PoolingType;

    use super::OpenAICombinedResponseTransformer;
    use super::OpenAIStreamingResponseTransformer;
    use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;

    fn make_token_message(token_result: GeneratedTokenResult) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            request_id: "test-request".to_string(),
            response: OutgoingResponse::GeneratedToken(token_result),
        })
    }

    fn make_embedding_message() -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            request_id: "test-request".to_string(),
            response: OutgoingResponse::Embedding(EmbeddingResult::Embedding(Embedding {
                embedding: vec![1.0],
                normalization_method: EmbeddingNormalizationMethod::None,
                pooling_type: PoolingType::Mean,
                source_document_id: "doc".to_string(),
            })),
        })
    }

    #[actix_web::test]
    async fn streaming_token_emits_content_delta() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = make_token_message(GeneratedTokenResult::Token("hello".to_string()));
        let result = transformer.transform(message).await?;

        assert_eq!(result["choices"][0]["delta"]["content"], "hello");
        assert_eq!(result["choices"][0]["delta"]["role"], "assistant");
        assert!(result["choices"][0]["finish_reason"].is_null());

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_done_emits_stop_finish_reason() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = make_token_message(GeneratedTokenResult::Done);
        let result = transformer.transform(message).await?;

        assert_eq!(result["choices"][0]["finish_reason"], "stop");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_token_returns_content() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = make_token_message(GeneratedTokenResult::Token("hello".to_string()));
        let result = transformer.transform(message).await?;

        assert_eq!(result, "hello");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_done_returns_empty_string() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = make_token_message(GeneratedTokenResult::Done);
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_chat_template_error_returns_error_json() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = make_token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_string(),
        ));
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["code"], "chat_template_error");
        assert_eq!(result["error"]["type"], "server_error");
        assert!(
            result["error"]["message"]
                .as_str()
                .unwrap()
                .contains("bad template")
        );

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_image_decoding_failed_returns_error_json() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = make_token_message(GeneratedTokenResult::ImageDecodingFailed(
            "bad image".to_string(),
        ));
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["code"], "image_decoding_failed");
        assert_eq!(result["error"]["type"], "server_error");
        assert!(
            result["error"]["message"]
                .as_str()
                .unwrap()
                .contains("bad image")
        );

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_multimodal_not_supported_returns_error_json() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = make_token_message(GeneratedTokenResult::MultimodalNotSupported(
            "no vision".to_string(),
        ));
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["code"], "multimodal_not_supported");
        assert_eq!(result["error"]["type"], "server_error");
        assert!(
            result["error"]["message"]
                .as_str()
                .unwrap()
                .contains("no vision")
        );

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_timeout_returns_error_json() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = OutgoingMessage::Response(ResponseEnvelope {
            request_id: "test-request".to_string(),
            response: OutgoingResponse::Timeout,
        });
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["code"], "timeout");
        assert_eq!(result["error"]["type"], "server_error");
        assert_eq!(result["error"]["message"], "inference timeout");

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_too_many_buffered_requests_returns_error_json() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = OutgoingMessage::Response(ResponseEnvelope {
            request_id: "test-request".to_string(),
            response: OutgoingResponse::TooManyBufferedRequests,
        });
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["code"], "too_many_buffered_requests");
        assert_eq!(result["error"]["type"], "server_error");
        assert_eq!(result["error"]["message"], "too many buffered requests");

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_embedding_returns_error_json() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = make_embedding_message();
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["code"], "unexpected_embedding_response");
        assert_eq!(result["error"]["type"], "server_error");
        assert_eq!(result["error"]["message"], "unexpected embedding response");

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_inference_error_returns_error_json() -> anyhow::Result<()> {
        let transformer = OpenAIStreamingResponseTransformer {
            model: "test-model".to_string(),
            system_fingerprint: "test-fingerprint".to_string(),
        };

        let message = OutgoingMessage::Error(ErrorEnvelope {
            request_id: "test-request".to_string(),
            error: JsonRpcError {
                code: 0,
                description: "something went wrong".to_string(),
            },
        });
        let result = transformer.transform(message).await?;

        assert_eq!(result["error"]["code"], "inference_error");
        assert_eq!(result["error"]["type"], "server_error");
        assert!(
            result["error"]["message"]
                .as_str()
                .unwrap()
                .contains("something went wrong")
        );

        Ok(())
    }

    #[actix_web::test]
    async fn combined_chat_template_error_captures_error() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = make_token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_string(),
        ));
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        let error_guard = transformer
            .captured_error
            .lock()
            .expect("Poisoned error lock");
        let error = error_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Expected error to be captured"))?;

        assert_eq!(error["error"]["code"], "chat_template_error");
        assert_eq!(error["error"]["type"], "server_error");
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("bad template")
        );

        Ok(())
    }

    #[actix_web::test]
    async fn combined_image_decoding_failed_captures_error() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = make_token_message(GeneratedTokenResult::ImageDecodingFailed(
            "bad image".to_string(),
        ));
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        let error_guard = transformer
            .captured_error
            .lock()
            .expect("Poisoned error lock");
        let error = error_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Expected error to be captured"))?;

        assert_eq!(error["error"]["code"], "image_decoding_failed");
        assert_eq!(error["error"]["type"], "server_error");
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("bad image")
        );

        Ok(())
    }

    #[actix_web::test]
    async fn combined_multimodal_not_supported_captures_error() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = make_token_message(GeneratedTokenResult::MultimodalNotSupported(
            "no vision".to_string(),
        ));
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        let error_guard = transformer
            .captured_error
            .lock()
            .expect("Poisoned error lock");
        let error = error_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Expected error to be captured"))?;

        assert_eq!(error["error"]["code"], "multimodal_not_supported");
        assert_eq!(error["error"]["type"], "server_error");
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("no vision")
        );

        Ok(())
    }

    #[actix_web::test]
    async fn combined_timeout_captures_error() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = OutgoingMessage::Response(ResponseEnvelope {
            request_id: "test-request".to_string(),
            response: OutgoingResponse::Timeout,
        });
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        let error_guard = transformer
            .captured_error
            .lock()
            .expect("Poisoned error lock");
        let error = error_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Expected error to be captured"))?;

        assert_eq!(error["error"]["code"], "timeout");
        assert_eq!(error["error"]["type"], "server_error");
        assert_eq!(error["error"]["message"], "inference timeout");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_too_many_buffered_requests_captures_error() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = OutgoingMessage::Response(ResponseEnvelope {
            request_id: "test-request".to_string(),
            response: OutgoingResponse::TooManyBufferedRequests,
        });
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        let error_guard = transformer
            .captured_error
            .lock()
            .expect("Poisoned error lock");
        let error = error_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Expected error to be captured"))?;

        assert_eq!(error["error"]["code"], "too_many_buffered_requests");
        assert_eq!(error["error"]["type"], "server_error");
        assert_eq!(error["error"]["message"], "too many buffered requests");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_embedding_captures_error() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = make_embedding_message();
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        let error_guard = transformer
            .captured_error
            .lock()
            .expect("Poisoned error lock");
        let error = error_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Expected error to be captured"))?;

        assert_eq!(error["error"]["code"], "unexpected_embedding_response");
        assert_eq!(error["error"]["type"], "server_error");
        assert_eq!(error["error"]["message"], "unexpected embedding response");

        Ok(())
    }

    #[actix_web::test]
    async fn combined_inference_error_captures_error() -> anyhow::Result<()> {
        let transformer = OpenAICombinedResponseTransformer {
            captured_error: Arc::new(Mutex::new(None)),
        };

        let message = OutgoingMessage::Error(ErrorEnvelope {
            request_id: "test-request".to_string(),
            error: JsonRpcError {
                code: 0,
                description: "something went wrong".to_string(),
            },
        });
        let result = transformer.transform(message).await?;

        assert_eq!(result, "");

        let error_guard = transformer
            .captured_error
            .lock()
            .expect("Poisoned error lock");
        let error = error_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Expected error to be captured"))?;

        assert_eq!(error["error"]["code"], "inference_error");
        assert_eq!(error["error"]["type"], "server_error");
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("something went wrong")
        );

        Ok(())
    }
}
