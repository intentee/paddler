use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use nanoid::nanoid;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::generation_summary::GenerationSummary;
use paddler_types::inference_client::Message as OutgoingMessage;
use paddler_types::inference_client::Response as OutgoingResponse;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::jsonrpc::ResponseEnvelope;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_types::token_usage::TokenUsage;
use paddler_types::validates::Validates;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::StreamExt as _;

use crate::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
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

fn openai_usage_json(usage: &TokenUsage) -> serde_json::Value {
    json!({
        "prompt_tokens": usage.prompt_tokens,
        "completion_tokens": usage.completion_tokens(),
        "total_tokens": usage.total_tokens(),
        "prompt_tokens_details": {
            "cached_tokens": usage.cached_prompt_tokens,
            "audio_tokens": usage.input_audio_tokens,
            "image_tokens": usage.input_image_tokens,
        },
        "completion_tokens_details": {
            "reasoning_tokens": usage.reasoning_tokens,
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

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct StreamOptions {
    #[serde(default)]
    include_usage: bool,
}

#[derive(Deserialize)]
struct OpenAICompletionRequestParams {
    max_completion_tokens: Option<i32>,
    messages: Vec<OpenAIMessage>,
    /// This parameter is ignored here, but is required by the `OpenAI` API.
    model: String,
    stream: Option<bool>,
    stream_options: Option<StreamOptions>,
    #[serde(default)]
    tools: Vec<Tool<RawParametersSchema>>,
}

#[derive(Clone)]
struct OpenAIStreamingResponseTransformer {
    include_usage: bool,
    model: String,
    system_fingerprint: String,
}

impl OpenAIStreamingResponseTransformer {
    fn content_chunk(&self, request_id: &str, text: &str) -> Result<String> {
        Ok(serde_json::to_string(&json!({
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
                        "content": text,
                    },
                    "logprobs": null,
                    "finish_reason": null
                }
            ]
        }))?)
    }

    fn reasoning_chunk(&self, request_id: &str, text: &str) -> Result<String> {
        Ok(serde_json::to_string(&json!({
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
                        "reasoning_content": text,
                    },
                    "logprobs": null,
                    "finish_reason": null
                }
            ]
        }))?)
    }

    fn tool_call_arguments_chunk(&self, request_id: &str, text: &str) -> Result<String> {
        Ok(serde_json::to_string(&json!({
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
                        "tool_calls": [
                            {
                                "index": 0,
                                "type": "function",
                                "function": {
                                    "arguments": text,
                                }
                            }
                        ],
                    },
                    "logprobs": null,
                    "finish_reason": null
                }
            ]
        }))?)
    }

    fn finish_chunk(&self, request_id: &str) -> Result<String> {
        Ok(serde_json::to_string(&json!({
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
        }))?)
    }

    fn usage_chunk(&self, request_id: &str, usage: &TokenUsage) -> Result<String> {
        Ok(serde_json::to_string(&json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": current_timestamp(),
            "model": self.model,
            "system_fingerprint": self.system_fingerprint,
            "choices": [],
            "usage": openai_usage_json(usage),
        }))?)
    }
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAIStreamingResponseTransformer {
    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ContentToken(text)
                        | GeneratedTokenResult::UndeterminableToken(text),
                    ),
            }) => Ok(vec![TransformResult::Chunk(
                self.content_chunk(&request_id, &text)?,
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::ReasoningToken(text)),
            }) => Ok(vec![TransformResult::Chunk(
                self.reasoning_chunk(&request_id, &text)?,
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallToken(text)),
            }) => Ok(vec![TransformResult::Chunk(
                self.tool_call_arguments_chunk(&request_id, &text)?,
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done(summary)),
            }) => {
                let finish = TransformResult::Chunk(self.finish_chunk(&request_id)?);

                if self.include_usage {
                    let usage = TransformResult::Chunk(
                        self.usage_chunk(&request_id, &summary.usage)?,
                    );

                    Ok(vec![finish, usage])
                } else {
                    Ok(vec![finish])
                }
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ChatTemplateError(description)
                        | GeneratedTokenResult::GrammarIncompatibleWithThinking(description)
                        | GeneratedTokenResult::GrammarRejectedModelOutput(description)
                        | GeneratedTokenResult::GrammarInitializationFailed(description)
                        | GeneratedTokenResult::GrammarSyntaxError(description)
                        | GeneratedTokenResult::ImageDecodingFailed(description)
                        | GeneratedTokenResult::MultimodalNotSupported(description)
                        | GeneratedTokenResult::SamplerError(description),
                    ),
                ..
            })
            | OutgoingMessage::Error(ErrorEnvelope {
                error: paddler_types::jsonrpc::Error { description, .. },
                ..
            }) => Ok(vec![TransformResult::Error(
                openai_error_json("server_error", &description).to_string(),
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Timeout,
                ..
            }) => Ok(vec![TransformResult::Error(
                openai_error_json("timeout", "request timed out").to_string(),
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::TooManyBufferedRequests,
                ..
            }) => Ok(vec![TransformResult::Error(
                openai_error_json("rate_limit_error", "too many buffered requests").to_string(),
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Embedding(_),
                ..
            }) => Ok(vec![TransformResult::Error(
                openai_error_json(
                    "invalid_request_error",
                    "unexpected embedding response in chat completions",
                )
                .to_string(),
            )]),
        }
    }
}

struct ToolCallPayload {
    name: String,
    arguments: String,
}

fn parse_tool_call_payload(buffer: &str) -> ToolCallPayload {
    let trimmed = buffer.trim();

    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return ToolCallPayload {
            name: String::new(),
            arguments: trimmed.to_owned(),
        };
    };

    let name = parsed
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let arguments = match parsed.get("arguments") {
        Some(value) => serde_json::to_string(value).unwrap_or_default(),
        None => String::new(),
    };

    ToolCallPayload { name, arguments }
}

#[derive(Default)]
struct OpenAINonStreamingState {
    content: String,
    reasoning: String,
    tool_call: String,
    summary: Option<GenerationSummary>,
}

#[derive(Clone)]
struct OpenAINonStreamingResponseTransformer {
    model: String,
    state: Arc<Mutex<OpenAINonStreamingState>>,
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAINonStreamingResponseTransformer {
    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ContentToken(text)
                        | GeneratedTokenResult::UndeterminableToken(text),
                    ),
                ..
            }) => {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|err| anyhow!("non-streaming state mutex poisoned: {err}"))?;

                state.content.push_str(&text);

                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::ReasoningToken(text)),
                ..
            }) => {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|err| anyhow!("non-streaming state mutex poisoned: {err}"))?;

                state.reasoning.push_str(&text);

                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallToken(text)),
                ..
            }) => {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|err| anyhow!("non-streaming state mutex poisoned: {err}"))?;

                state.tool_call.push_str(&text);

                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done(summary)),
            }) => {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|err| anyhow!("non-streaming state mutex poisoned: {err}"))?;

                state.summary = Some(summary);

                let mut message_obj = json!({
                    "role": "assistant",
                    "content": state.content,
                    "refusal": null,
                    "annotations": []
                });

                if !state.reasoning.is_empty()
                    && let Some(map) = message_obj.as_object_mut()
                {
                    map.insert("reasoning_content".to_owned(), json!(state.reasoning));
                }

                let finish_reason = if state.tool_call.is_empty() {
                    "stop"
                } else {
                    let parsed_tool_call = parse_tool_call_payload(&state.tool_call);

                    if let Some(map) = message_obj.as_object_mut() {
                        map.insert(
                            "content".to_owned(),
                            if state.content.is_empty() {
                                serde_json::Value::Null
                            } else {
                                json!(state.content)
                            },
                        );
                        map.insert(
                            "tool_calls".to_owned(),
                            json!([{
                                "id": format!("call_{}", nanoid!()),
                                "type": "function",
                                "function": {
                                    "name": parsed_tool_call.name,
                                    "arguments": parsed_tool_call.arguments,
                                }
                            }]),
                        );
                    }

                    "tool_calls"
                };

                let body = serde_json::to_string(&json!({
                    "id": request_id,
                    "object": "chat.completion",
                    "created": current_timestamp(),
                    "model": self.model,
                    "choices": [
                        {
                            "index": 0,
                            "message": message_obj,
                            "logprobs": null,
                            "finish_reason": finish_reason
                        }
                    ],
                    "usage": openai_usage_json(&summary.usage),
                    "service_tier": "default"
                }))?;

                Ok(vec![TransformResult::Chunk(body)])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ChatTemplateError(description)
                        | GeneratedTokenResult::GrammarIncompatibleWithThinking(description)
                        | GeneratedTokenResult::GrammarRejectedModelOutput(description)
                        | GeneratedTokenResult::GrammarInitializationFailed(description)
                        | GeneratedTokenResult::GrammarSyntaxError(description)
                        | GeneratedTokenResult::ImageDecodingFailed(description)
                        | GeneratedTokenResult::MultimodalNotSupported(description)
                        | GeneratedTokenResult::SamplerError(description),
                    ),
                ..
            })
            | OutgoingMessage::Error(ErrorEnvelope {
                error: paddler_types::jsonrpc::Error { description, .. },
                ..
            }) => Ok(vec![TransformResult::Error(
                openai_error_json("server_error", &description).to_string(),
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Timeout,
                ..
            }) => Ok(vec![TransformResult::Error(
                openai_error_json("timeout", "request timed out").to_string(),
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::TooManyBufferedRequests,
                ..
            }) => Ok(vec![TransformResult::Error(
                openai_error_json("rate_limit_error", "too many buffered requests").to_string(),
            )]),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::Embedding(_),
                ..
            }) => Ok(vec![TransformResult::Error(
                openai_error_json(
                    "invalid_request_error",
                    "unexpected embedding response in chat completions",
                )
                .to_string(),
            )]),
        }
    }
}

#[post("/v1/chat/completions")]
async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAICompletionRequestParams>,
) -> Result<HttpResponse, Error> {
    let openai_params = openai_params.into_inner();

    let validated_tools = match openai_params
        .tools
        .into_iter()
        .map(Validates::validate)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(tools) => tools,
        Err(err) => {
            return Ok(HttpResponse::BadRequest()
                .content_type("application/json")
                .body(
                    openai_error_json("invalid_request_error", &err.to_string()).to_string(),
                ));
        }
    };

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
        grammar: None,
        max_tokens: openai_params.max_completion_tokens.unwrap_or(2000),
        tools: validated_tools,
    };

    if openai_params.stream.unwrap_or(false) {
        let include_usage = openai_params
            .stream_options
            .as_ref()
            .map_or(false, |options| options.include_usage);

        Ok(http_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAIStreamingResponseTransformer {
                include_usage,
                model: openai_params.model.clone(),
                system_fingerprint: nanoid!(),
            },
        ))
    } else {
        let results: Vec<TransformResult> = unbounded_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAINonStreamingResponseTransformer {
                model: openai_params.model.clone(),
                state: Arc::new(Mutex::new(OpenAINonStreamingState::default())),
            },
        )
        .collect()
        .await;

        if let Some(TransformResult::Error(error_json)) = results
            .iter()
            .find(|result| matches!(result, TransformResult::Error(_)))
        {
            return Ok(HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(error_json.clone()));
        }

        let body = results
            .into_iter()
            .find_map(|result| match result {
                TransformResult::Chunk(content) => Some(content),
                TransformResult::Discard | TransformResult::Error(_) => None,
            });

        match body {
            Some(json_body) => Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(json_body)),
            None => Ok(HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(
                    openai_error_json("server_error", "no completion produced").to_string(),
                )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;

    use anyhow::Result;
    use paddler_types::generated_token_result::GeneratedTokenResult;
    use paddler_types::generation_summary::GenerationSummary;
    use paddler_types::inference_client::Message as OutgoingMessage;
    use paddler_types::inference_client::Response as OutgoingResponse;
    use paddler_types::jsonrpc::ErrorEnvelope;
    use paddler_types::jsonrpc::ResponseEnvelope;
    use paddler_types::token_usage::TokenUsage;

    use super::OpenAINonStreamingResponseTransformer;
    use super::OpenAINonStreamingState;
    use super::OpenAIStreamingResponseTransformer;
    use crate::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
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

    fn streaming_transformer(include_usage: bool) -> OpenAIStreamingResponseTransformer {
        OpenAIStreamingResponseTransformer {
            include_usage,
            model: "test-model".to_owned(),
            system_fingerprint: "test-fingerprint".to_owned(),
        }
    }

    fn non_streaming_transformer() -> OpenAINonStreamingResponseTransformer {
        OpenAINonStreamingResponseTransformer {
            model: "test-model".to_owned(),
            state: Arc::new(Mutex::new(OpenAINonStreamingState::default())),
        }
    }

    fn assert_chunk_contains(result: &TransformResult, expected: &str) -> Result<()> {
        let TransformResult::Chunk(content) = result else {
            anyhow::bail!("expected TransformResult::Chunk, got TransformResult::Error");
        };

        assert!(
            content.contains(expected),
            "chunk does not contain '{expected}': {content}"
        );

        Ok(())
    }

    fn assert_chunk_does_not_contain(result: &TransformResult, expected: &str) -> Result<()> {
        let TransformResult::Chunk(content) = result else {
            anyhow::bail!("expected TransformResult::Chunk, got TransformResult::Error");
        };

        assert!(
            !content.contains(expected),
            "chunk unexpectedly contains '{expected}': {content}"
        );

        Ok(())
    }

    fn assert_error_contains(result: &TransformResult, expected: &str) -> Result<()> {
        let TransformResult::Error(content) = result else {
            anyhow::bail!("expected TransformResult::Error, got TransformResult::Chunk");
        };

        assert!(
            content.contains(expected),
            "error does not contain '{expected}': {content}"
        );

        Ok(())
    }

    fn summary_with_counts(
        prompt_tokens: u64,
        content_tokens: u64,
        reasoning_tokens: u64,
    ) -> GenerationSummary {
        GenerationSummary {
            usage: TokenUsage {
                prompt_tokens,
                content_tokens,
                reasoning_tokens,
                ..TokenUsage::default()
            },
        }
    }

    #[actix_web::test]
    async fn streaming_content_token_emits_content_delta() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_token_message(GeneratedTokenResult::ContentToken("hello".to_owned()));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"content\":\"hello\"")?;
        assert_chunk_contains(&chunks[0], "\"role\":\"assistant\"")?;
        assert_chunk_does_not_contain(&chunks[0], "reasoning_content")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_reasoning_token_emits_reasoning_content_delta() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_token_message(GeneratedTokenResult::ReasoningToken("thought".to_owned()));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"reasoning_content\":\"thought\"")?;
        assert_chunk_contains(&chunks[0], "\"role\":\"assistant\"")?;
        assert_chunk_does_not_contain(&chunks[0], "\"content\":")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_undeterminable_token_emits_content_delta() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message =
            make_token_message(GeneratedTokenResult::UndeterminableToken("ambig".to_owned()));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"content\":\"ambig\"")?;
        assert_chunk_does_not_contain(&chunks[0], "reasoning_content")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_done_without_include_usage_emits_only_finish_chunk() -> Result<()> {
        let transformer = streaming_transformer(false);
        let summary = summary_with_counts(5, 3, 2);

        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"")?;
        assert_chunk_does_not_contain(&chunks[0], "usage")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_done_with_include_usage_emits_finish_then_usage_chunk() -> Result<()> {
        let transformer = streaming_transformer(true);
        let summary = summary_with_counts(7, 4, 1);

        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(chunks.len(), 2);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"")?;
        assert_chunk_does_not_contain(&chunks[0], "usage")?;
        assert_chunk_contains(&chunks[1], "\"prompt_tokens\":7")?;
        assert_chunk_contains(&chunks[1], "\"completion_tokens\":5")?;
        assert_chunk_contains(&chunks[1], "\"reasoning_tokens\":1")?;
        assert_chunk_contains(&chunks[1], "\"total_tokens\":12")?;
        assert_chunk_contains(&chunks[1], "\"choices\":[]")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_tool_call_token_emits_tool_calls_arguments_delta() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_token_message(GeneratedTokenResult::ToolCallToken(
            "{\"name\":\"get_weather\"}".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"tool_calls\"")?;
        assert_chunk_contains(&chunks[0], "\"function\"")?;
        assert_chunk_contains(&chunks[0], "\"arguments\":\"{\\\"name\\\":\\\"get_weather\\\"}\"")?;
        assert_chunk_does_not_contain(&chunks[0], "\"content\":")?;
        assert_chunk_does_not_contain(&chunks[0], "reasoning_content")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_tool_call_aggregates_into_message_tool_calls() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallToken(
                "{\"name\":\"get_weather\",\"arguments\":{\"location\":\"Paris\"}}".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(4, 0, 0);
        let final_chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"tool_calls\":")?;
        assert_chunk_contains(&final_chunks[0], "\"name\":\"get_weather\"")?;
        assert_chunk_contains(
            &final_chunks[0],
            "\"arguments\":\"{\\\"location\\\":\\\"Paris\\\"}\"",
        )?;
        assert_chunk_contains(&final_chunks[0], "\"finish_reason\":\"tool_calls\"")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_unparseable_tool_call_falls_back_to_raw_arguments() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallToken(
                "garbage payload".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(2, 0, 0);
        let final_chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"tool_calls\":")?;
        assert_chunk_contains(&final_chunks[0], "\"arguments\":\"garbage payload\"")?;

        Ok(())
    }

    #[test]
    fn parse_tool_call_payload_extracts_name_and_arguments() {
        let parsed = super::parse_tool_call_payload(
            "{\"name\":\"get_weather\",\"arguments\":{\"location\":\"Paris\",\"unit\":\"c\"}}",
        );

        assert_eq!(parsed.name, "get_weather");
        assert_eq!(parsed.arguments, "{\"location\":\"Paris\",\"unit\":\"c\"}");
    }

    #[test]
    fn parse_tool_call_payload_handles_whitespace_around_payload() {
        let parsed =
            super::parse_tool_call_payload("\n  {\"name\":\"x\",\"arguments\":{}}  \n");

        assert_eq!(parsed.name, "x");
        assert_eq!(parsed.arguments, "{}");
    }

    #[test]
    fn parse_tool_call_payload_returns_raw_arguments_when_invalid_json() {
        let parsed = super::parse_tool_call_payload("not even close to JSON");

        assert_eq!(parsed.name, "");
        assert_eq!(parsed.arguments, "not even close to JSON");
    }

    #[test]
    fn parse_tool_call_payload_with_missing_arguments_returns_empty_string() {
        let parsed = super::parse_tool_call_payload("{\"name\":\"x\"}");

        assert_eq!(parsed.name, "x");
        assert_eq!(parsed.arguments, "");
    }

    #[actix_web::test]
    async fn streaming_error_message_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_error_message(500, "internal server error");
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "internal server error")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_chat_template_error_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_token_message(GeneratedTokenResult::ChatTemplateError(
            "bad template".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "bad template")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_timeout_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_response_message(OutgoingResponse::Timeout);
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "request timed out")?;
        assert_error_contains(&chunks[0], "timeout")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_too_many_buffered_requests_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_response_message(OutgoingResponse::TooManyBufferedRequests);
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "too many buffered requests")?;
        assert_error_contains(&chunks[0], "rate_limit_error")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_image_decoding_failed_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_token_message(GeneratedTokenResult::ImageDecodingFailed(
            "unsupported format".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "unsupported format")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_multimodal_not_supported_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_token_message(GeneratedTokenResult::MultimodalNotSupported(
            "model does not support images".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "model does not support images")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_aggregates_content_only_when_no_reasoning() -> Result<()> {
        let transformer = non_streaming_transformer();

        assert_eq!(
            transformer
                .transform(make_token_message(GeneratedTokenResult::ContentToken(
                    "hel".to_owned()
                )))
                .await?
                .len(),
            0
        );
        assert_eq!(
            transformer
                .transform(make_token_message(GeneratedTokenResult::ContentToken(
                    "lo".to_owned()
                )))
                .await?
                .len(),
            0
        );

        let summary = summary_with_counts(4, 2, 0);
        let final_chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"content\":\"hello\"")?;
        assert_chunk_does_not_contain(&final_chunks[0], "reasoning_content")?;
        assert_chunk_contains(&final_chunks[0], "\"prompt_tokens\":4")?;
        assert_chunk_contains(&final_chunks[0], "\"completion_tokens\":2")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_separates_reasoning_from_content() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(make_token_message(GeneratedTokenResult::ReasoningToken(
                "think".to_owned(),
            )))
            .await?;
        transformer
            .transform(make_token_message(GeneratedTokenResult::ContentToken(
                "answer".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(3, 1, 1);
        let final_chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"content\":\"answer\"")?;
        assert_chunk_contains(&final_chunks[0], "\"reasoning_content\":\"think\"")?;
        assert_chunk_contains(&final_chunks[0], "\"reasoning_tokens\":1")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_undeterminable_routes_to_content() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(make_token_message(GeneratedTokenResult::UndeterminableToken(
                "amb".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(2, 0, 0);
        let final_chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_contains(&final_chunks[0], "\"content\":\"amb\"")?;
        assert_chunk_does_not_contain(&final_chunks[0], "reasoning_content")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_error_message_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(make_error_message(500, "internal server error"))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "internal server error")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[test]
    fn deserialize_text_only_request() -> Result<()> {
        let input = serde_json::json!({
            "model": "test-model",
            "messages": [
                {"role": "user", "content": "hello"}
            ]
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input)?;

        assert_eq!(params.model, "test-model");
        assert_eq!(params.messages.len(), 1);
        assert_eq!(params.messages[0].role, "user");
        assert_eq!(params.messages[0].content.text_content(), "hello");

        Ok(())
    }

    #[test]
    fn deserialize_request_with_stream_options_include_usage_true() -> Result<()> {
        let input = serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true,
            "stream_options": {"include_usage": true}
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input)?;

        let stream_options = params
            .stream_options
            .ok_or_else(|| anyhow::anyhow!("expected stream_options"))?;

        assert!(stream_options.include_usage);

        Ok(())
    }

    #[test]
    fn deserialize_request_without_stream_options_defaults_to_none() -> Result<()> {
        let input = serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input)?;

        assert!(params.stream_options.is_none());

        Ok(())
    }

    #[test]
    fn deserialize_multimodal_request_with_image() -> Result<()> {
        let input = serde_json::json!({
            "model": "vision-model",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "describe this image"},
                        {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/4AAQ"}}
                    ]
                }
            ]
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input)?;

        assert_eq!(params.messages.len(), 1);
        assert_eq!(
            params.messages[0].content.text_content(),
            "describe this image"
        );

        let image_urls = params.messages[0].content.image_urls();

        assert_eq!(image_urls.len(), 1);
        assert_eq!(image_urls[0].url, "data:image/jpeg;base64,/9j/4AAQ");

        Ok(())
    }

    #[test]
    fn deserialize_multi_turn_conversation() -> Result<()> {
        let input = serde_json::json!({
            "model": "test-model",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is 2+2?"},
                {"role": "assistant", "content": "4"},
                {"role": "user", "content": "And 3+3?"}
            ]
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input)?;

        assert_eq!(params.messages.len(), 4);
        assert_eq!(params.messages[0].role, "system");
        assert_eq!(params.messages[1].role, "user");
        assert_eq!(params.messages[2].role, "assistant");
        assert_eq!(params.messages[3].role, "user");

        Ok(())
    }

    #[test]
    fn openai_message_converts_to_conversation_message() -> Result<()> {
        use paddler_types::conversation_message::ConversationMessage;

        let input = serde_json::json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "OCR this"},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc"}}
            ]
        });

        let openai_message: super::OpenAIMessage = serde_json::from_value(input)?;
        let conversation_message = ConversationMessage::from(&openai_message);

        assert_eq!(conversation_message.role, "user");
        assert_eq!(conversation_message.content.text_content(), "OCR this");
        assert_eq!(conversation_message.content.image_urls().len(), 1);

        Ok(())
    }

    #[test]
    fn openai_error_json_has_correct_structure() -> Result<()> {
        let error = super::openai_error_json("server_error", "something went wrong");

        assert_eq!(error["error"]["type"], "server_error");
        assert_eq!(error["error"]["message"], "something went wrong");
        assert!(error["error"]["param"].is_null());
        assert!(error["error"]["code"].is_null());

        Ok(())
    }
}
