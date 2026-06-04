use std::sync::Arc;
use std::time::SystemTime;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use nanoid::nanoid;
use parking_lot::Mutex;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use llama_cpp_bindings_types::ParsedToolCall;
use llama_cpp_bindings_types::TokenUsage;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::validates::Validates;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::StreamExt as _;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::compatibility::openai_service::chat_completions_sse_response::chat_completions_sse_response;
use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::timestamp_from::timestamp_from;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

fn openai_usage_json(usage: &TokenUsage) -> serde_json::Value {
    json!({
        "prompt_tokens": usage.prompt_tokens,
        "completion_tokens": usage.completion_tokens(),
        "total_tokens": usage.total_tokens(),
        "prompt_tokens_details": {
            "cached_tokens": usage.cached_prompt_tokens,
            "audio_tokens": usage.input_audio_tokens,
        },
        "completion_tokens_details": {
            "reasoning_tokens": usage.reasoning_tokens,
        }
    })
}

fn try_universal_error_chunk(message: &OutgoingMessage) -> Option<TransformResult> {
    if let OutgoingMessage::Response(ResponseEnvelope {
        response: OutgoingResponse::Embedding(_),
        ..
    }) = message
    {
        return Some(TransformResult::Error(
            OpenAIError {
                error_type: "invalid_request_error",
                message: "unexpected embedding response in chat completions".to_owned(),
            }
            .to_envelope()
            .to_string(),
        ));
    }

    OpenAIError::classify(message)
        .map(|error| TransformResult::Error(error.to_envelope().to_string()))
}

#[derive(Deserialize)]
struct OpenAIMessage {
    content: ConversationMessageContent,
    role: String,
}

impl OpenAIMessage {
    fn to_conversation_message(&self) -> ConversationMessage {
        ConversationMessage {
            content: self.content.clone(),
            role: self.role.clone(),
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

#[derive(Default)]
struct OpenAIStreamingState {
    saw_tool_call: bool,
}

#[derive(Clone)]
struct OpenAIStreamingResponseTransformer {
    created: u64,
    include_usage: bool,
    model: String,
    state: Arc<Mutex<OpenAIStreamingState>>,
    system_fingerprint: String,
}

impl OpenAIStreamingResponseTransformer {
    fn content_chunk(&self, request_id: &str, text: &str) -> Result<String> {
        serde_json::to_string(&json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": self.created,
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
        }))
        .context("serializing content chunk")
    }

    fn tool_calls_chunk(
        &self,
        request_id: &str,
        parsed_calls: &[ParsedToolCall],
    ) -> Result<String> {
        parsed_calls
            .iter()
            .enumerate()
            .map(|(index, call)| {
                arguments_to_tool_call_string(&call.arguments).map(|arguments| {
                    json!({
                        "index": index,
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": arguments,
                        }
                    })
                })
            })
            .collect::<Result<Vec<_>>>()
            .and_then(|tool_calls| {
                serde_json::to_string(&json!({
                    "id": request_id,
                    "object": "chat.completion.chunk",
                    "created": self.created,
                    "model": self.model,
                    "system_fingerprint": self.system_fingerprint,
                    "choices": [
                        {
                            "index": 0,
                            "delta": {
                                "role": "assistant",
                                "tool_calls": tool_calls,
                            },
                            "logprobs": null,
                            "finish_reason": null
                        }
                    ]
                }))
                .context("serializing tool-calls chunk")
            })
    }

    fn finish_chunk(&self, request_id: &str, finish_reason: &str) -> Result<String> {
        serde_json::to_string(&json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "system_fingerprint": self.system_fingerprint,
            "choices": [
                {
                    "index": 0,
                    "delta": {},
                    "logprobs": null,
                    "finish_reason": finish_reason
                }
            ]
        }))
        .context("serializing finish chunk")
    }

    fn usage_chunk(&self, request_id: &str, usage: &TokenUsage) -> Result<String> {
        serde_json::to_string(&json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "system_fingerprint": self.system_fingerprint,
            "choices": [],
            "usage": openai_usage_json(usage),
        }))
        .context("serializing usage chunk")
    }

    fn handle_content(&self, request_id: &str, text: &str) -> Result<Vec<TransformResult>> {
        self.content_chunk(request_id, text)
            .map(|chunk| vec![TransformResult::Chunk(chunk)])
    }

    fn handle_tool_call_parsed(
        &self,
        request_id: &str,
        parsed_calls: &[ParsedToolCall],
    ) -> Result<Vec<TransformResult>> {
        if parsed_calls.is_empty() {
            return Ok(vec![]);
        }

        self.state.lock().saw_tool_call = true;

        self.tool_calls_chunk(request_id, parsed_calls)
            .map(|chunk| vec![TransformResult::Chunk(chunk)])
    }

    fn handle_done(
        &self,
        request_id: &str,
        summary: &GenerationSummary,
    ) -> Result<Vec<TransformResult>> {
        let saw_tool_call = self.state.lock().saw_tool_call;
        let finish_reason = if saw_tool_call { "tool_calls" } else { "stop" };

        self.finish_chunk(request_id, finish_reason)
            .and_then(|finish_chunk| {
                let finish = TransformResult::Chunk(finish_chunk);

                if self.include_usage {
                    self.usage_chunk(request_id, &summary.usage)
                        .map(|usage_chunk| vec![finish, TransformResult::Chunk(usage_chunk)])
                } else {
                    Ok(vec![finish])
                }
            })
    }
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAIStreamingResponseTransformer {
    type Output = TransformResult;

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        if let Some(error_chunk) = try_universal_error_chunk(&message) {
            return Ok(vec![error_chunk]);
        }

        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ContentToken(text)
                        | GeneratedTokenResult::UndeterminableToken(text),
                    ),
                ..
            }) => self.handle_content(&request_id, &text),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ReasoningToken(_)
                        | GeneratedTokenResult::ToolCallToken(_),
                    ),
                ..
            }) => Ok(vec![]),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallParsed(parsed_calls)),
                ..
            }) => self.handle_tool_call_parsed(&request_id, &parsed_calls),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done(summary)),
                ..
            }) => self.handle_done(&request_id, &summary),
            other => Err(anyhow!(
                "OpenAIStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
            )),
        }
    }
}

#[derive(Clone, Default)]
struct OpenAINonStreamingState {
    content: String,
    tool_calls: Vec<ParsedToolCall>,
}

#[derive(Clone)]
struct OpenAINonStreamingResponseTransformer {
    created: u64,
    model: String,
    state: Arc<Mutex<OpenAINonStreamingState>>,
}

impl OpenAINonStreamingResponseTransformer {
    fn append_content(&self, text: &str) {
        self.state.lock().content.push_str(text);
    }

    fn append_tool_calls(&self, parsed_calls: Vec<ParsedToolCall>) {
        self.state.lock().tool_calls.extend(parsed_calls);
    }

    fn build_done_chunk(&self, request_id: &str, summary: &GenerationSummary) -> Result<String> {
        let snapshot = self.snapshot_state();

        let has_tool_calls = !snapshot.tool_calls.is_empty();
        let finish_reason = if has_tool_calls { "tool_calls" } else { "stop" };

        let tool_calls_json = snapshot
            .tool_calls
            .iter()
            .map(|call| {
                arguments_to_tool_call_string(&call.arguments).map(|arguments| {
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": arguments,
                        }
                    })
                })
            })
            .collect::<Result<Vec<_>>>();

        tool_calls_json.and_then(|tool_calls_json| {
            let mut message_obj = json!({
                "role": "assistant",
                "content": if snapshot.content.is_empty() && has_tool_calls {
                    serde_json::Value::Null
                } else {
                    json!(snapshot.content)
                },
                "refusal": null,
                "annotations": []
            });

            if has_tool_calls && let Some(map) = message_obj.as_object_mut() {
                map.insert("tool_calls".to_owned(), json!(tool_calls_json));
            }

            serde_json::to_string(&json!({
                "id": request_id,
                "object": "chat.completion",
                "created": self.created,
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
            }))
            .context("serializing non-streaming completion")
        })
    }

    fn snapshot_state(&self) -> OpenAINonStreamingState {
        self.state.lock().clone()
    }
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAINonStreamingResponseTransformer {
    type Output = TransformResult;

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        if let Some(error_chunk) = try_universal_error_chunk(&message) {
            return Ok(vec![error_chunk]);
        }

        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ContentToken(text)
                        | GeneratedTokenResult::UndeterminableToken(text),
                    ),
                ..
            }) => {
                self.append_content(&text);
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(
                        GeneratedTokenResult::ReasoningToken(_)
                        | GeneratedTokenResult::ToolCallToken(_),
                    ),
                ..
            }) => Ok(vec![]),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallParsed(parsed_calls)),
                ..
            }) => {
                self.append_tool_calls(parsed_calls);
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::Done(summary)),
                ..
            }) => Ok(vec![TransformResult::Chunk(
                self.build_done_chunk(&request_id, &summary)?,
            )]),
            other => Err(anyhow!(
                "OpenAINonStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
            )),
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
                    OpenAIError {
                        error_type: "invalid_request_error",
                        message: err.to_string(),
                    }
                    .to_envelope()
                    .to_string(),
                ));
        }
    };

    let parse_tool_calls = !validated_tools.is_empty();
    let paddler_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(
            openai_params
                .messages
                .iter()
                .map(OpenAIMessage::to_conversation_message)
                .collect(),
        ),
        enable_thinking: true,
        grammar: None,
        max_tokens: openai_params.max_completion_tokens.unwrap_or(2000),
        parse_tool_calls,
        tools: validated_tools,
    };

    let created =
        timestamp_from(SystemTime::now()).map_err(actix_web::error::ErrorInternalServerError)?;

    if openai_params.stream.unwrap_or(false) {
        let include_usage = openai_params
            .stream_options
            .as_ref()
            .is_some_and(|options| options.include_usage);

        Ok(chat_completions_sse_response(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAIStreamingResponseTransformer {
                created,
                include_usage,
                model: openai_params.model.clone(),
                state: Arc::new(Mutex::new(OpenAIStreamingState::default())),
                system_fingerprint: nanoid!(),
            },
            app_data.shutdown.clone(),
        ))
    } else {
        let results: Vec<TransformResult> = unbounded_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAINonStreamingResponseTransformer {
                created,
                model: openai_params.model.clone(),
                state: Arc::new(Mutex::new(OpenAINonStreamingState::default())),
            },
            app_data.shutdown.clone(),
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

        let body = results.into_iter().find_map(|result| match result {
            TransformResult::Chunk(content) => Some(content),
            TransformResult::Discard | TransformResult::Error(_) => None,
        });

        Ok(body.map_or_else(
            || {
                HttpResponse::InternalServerError()
                    .content_type("application/json")
                    .body(
                        OpenAIError {
                            error_type: "server_error",
                            message: "no completion produced".to_owned(),
                        }
                        .to_envelope()
                        .to_string(),
                    )
            },
            |json_body| {
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(json_body)
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use parking_lot::Mutex;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::test::call_service;
    use actix_web::test::init_service;
    use actix_web::test::read_body;
    use actix_web::web::Data;
    use tokio_util::sync::CancellationToken;

    use anyhow::Result;
    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::AppData;
    use super::OpenAINonStreamingResponseTransformer;
    use super::OpenAINonStreamingState;
    use super::OpenAIStreamingResponseTransformer;
    use super::register;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;

    fn make_token_message(token_result: GeneratedTokenResult) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::GeneratedToken(token_result),
        })
    }

    fn make_error_message(code: i32, description: &str) -> OutgoingMessage {
        OutgoingMessage::Error(ErrorEnvelope {
            request_id: "test-request".to_owned(),
            error: paddler_messaging::jsonrpc::error::Error {
                code,
                description: description.to_owned(),
            },
        })
    }

    fn make_response_message(response: OutgoingResponse) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response,
        })
    }

    fn streaming_transformer(include_usage: bool) -> OpenAIStreamingResponseTransformer {
        OpenAIStreamingResponseTransformer {
            created: 0,
            include_usage,
            model: "test-model".to_owned(),
            state: Arc::new(Mutex::new(super::OpenAIStreamingState::default())),
            system_fingerprint: "test-fingerprint".to_owned(),
        }
    }

    fn non_streaming_transformer() -> OpenAINonStreamingResponseTransformer {
        OpenAINonStreamingResponseTransformer {
            created: 0,
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

    fn weather_call() -> ParsedToolCall {
        ParsedToolCall::new(
            "call_x".to_owned(),
            "get_weather".to_owned(),
            ToolCallArguments::ValidJson(serde_json::json!({"location": "Paris"})),
        )
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
    async fn streaming_reasoning_token_is_dropped() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message =
            make_token_message(GeneratedTokenResult::ReasoningToken("thought".to_owned()));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 0);

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_undeterminable_token_emits_content_delta() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_token_message(GeneratedTokenResult::UndeterminableToken(
            "ambig".to_owned(),
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"content\":\"ambig\"")?;
        assert_chunk_does_not_contain(&chunks[0], "reasoning_content")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_tool_call_token_is_silently_dropped() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallToken(
                "{".to_owned(),
            )))
            .await?;

        assert_eq!(chunks.len(), 0);

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_tool_call_parsed_emits_structured_tool_calls_chunk() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallParsed(
                vec![weather_call()],
            )))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"tool_calls\"")?;
        assert_chunk_contains(&chunks[0], "\"id\":\"call_x\"")?;
        assert_chunk_contains(&chunks[0], "\"name\":\"get_weather\"")?;
        assert_chunk_contains(
            &chunks[0],
            "\"arguments\":\"{\\\"location\\\":\\\"Paris\\\"}\"",
        )?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_done_after_tool_call_uses_tool_calls_finish_reason() -> Result<()> {
        let transformer = streaming_transformer(false);

        transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallParsed(
                vec![weather_call()],
            )))
            .await?;

        let summary = summary_with_counts(2, 0, 0);
        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"tool_calls\"")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_done_without_tool_call_uses_stop_finish_reason() -> Result<()> {
        let transformer = streaming_transformer(false);

        transformer
            .transform(make_token_message(GeneratedTokenResult::ContentToken(
                "hi".to_owned(),
            )))
            .await?;

        let summary = summary_with_counts(2, 1, 0);
        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(summary)))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_chunk_contains(&chunks[0], "\"finish_reason\":\"stop\"")?;

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
        assert_chunk_contains(&chunks[1], "\"total_tokens\":12")?;
        assert_chunk_contains(&chunks[1], "\"choices\":[]")?;

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
    async fn streaming_tool_call_parse_failed_emits_server_error() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::ToolCallParseFailed("bad payload".to_owned()),
            ))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "bad payload")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_tool_call_validation_failed_emits_server_error() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::ToolCallValidationFailed(vec!["missing field x".to_owned()]),
            ))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "missing field x")?;

        Ok(())
    }

    #[actix_web::test]
    async fn streaming_unrecognized_tool_call_format_emits_server_error() -> Result<()> {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::UnrecognizedToolCallFormat(
                    paddler_messaging::raw_tool_call_tokens::RawToolCallTokens {
                        text: "<unknown_marker>blah</unknown_marker>".to_owned(),
                        ffi_error_message: "common_chat_parse failed: no parser".to_owned(),
                    },
                ),
            ))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "common_chat_parse failed: no parser")?;
        assert_error_contains(&chunks[0], "<unknown_marker>blah</unknown_marker>")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
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
    async fn streaming_image_exceeds_batch_size_returns_error_variant() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message = make_token_message(GeneratedTokenResult::ImageExceedsBatchSize(
            paddler_messaging::oversized_image_details::OversizedImageDetails {
                image_tokens: 368,
                n_batch: 100,
            },
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "368")?;
        assert_error_contains(&chunks[0], "100")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_aggregates_content_only_when_no_reasoning() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(make_token_message(GeneratedTokenResult::ContentToken(
                "hel".to_owned(),
            )))
            .await?;
        transformer
            .transform(make_token_message(GeneratedTokenResult::ContentToken(
                "lo".to_owned(),
            )))
            .await?;

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
    async fn non_streaming_drops_reasoning_but_keeps_reasoning_token_count() -> Result<()> {
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
        assert_chunk_does_not_contain(&final_chunks[0], "reasoning_content")?;
        assert_chunk_contains(&final_chunks[0], "\"reasoning_tokens\":1")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_undeterminable_routes_to_content() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(make_token_message(
                GeneratedTokenResult::UndeterminableToken("amb".to_owned()),
            ))
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
    async fn non_streaming_tool_call_parsed_populates_message_tool_calls() -> Result<()> {
        let transformer = non_streaming_transformer();

        transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallParsed(
                vec![weather_call()],
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
    async fn non_streaming_tool_call_parse_failed_emits_error() -> Result<()> {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::ToolCallParseFailed("bad payload".to_owned()),
            ))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "bad payload")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_tool_call_validation_failed_emits_error() -> Result<()> {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::ToolCallValidationFailed(vec!["bad shape".to_owned()]),
            ))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "bad shape")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_unrecognized_tool_call_format_emits_server_error() -> Result<()> {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::UnrecognizedToolCallFormat(
                    paddler_messaging::raw_tool_call_tokens::RawToolCallTokens {
                        text: "<unknown_marker>blah</unknown_marker>".to_owned(),
                        ffi_error_message: "common_chat_parse failed: no parser".to_owned(),
                    },
                ),
            ))
            .await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "common_chat_parse failed: no parser")?;
        assert_error_contains(&chunks[0], "<unknown_marker>blah</unknown_marker>")?;
        assert_error_contains(&chunks[0], "server_error")?;

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

    #[actix_web::test]
    async fn non_streaming_chat_template_error_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_image_decoding_failed_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_multimodal_not_supported_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

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
    async fn non_streaming_image_exceeds_batch_size_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

        let message = make_token_message(GeneratedTokenResult::ImageExceedsBatchSize(
            paddler_messaging::oversized_image_details::OversizedImageDetails {
                image_tokens: 368,
                n_batch: 100,
            },
        ));
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "368")?;
        assert_error_contains(&chunks[0], "100")?;
        assert_error_contains(&chunks[0], "server_error")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_timeout_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

        let message = make_response_message(OutgoingResponse::Timeout);
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "request timed out")?;
        assert_error_contains(&chunks[0], "timeout")?;

        Ok(())
    }

    #[actix_web::test]
    async fn non_streaming_too_many_buffered_requests_returns_error_variant() -> Result<()> {
        let transformer = non_streaming_transformer();

        let message = make_response_message(OutgoingResponse::TooManyBufferedRequests);
        let chunks = transformer.transform(message).await?;

        assert_eq!(chunks.len(), 1);
        assert_error_contains(&chunks[0], "too many buffered requests")?;
        assert_error_contains(&chunks[0], "rate_limit_error")?;

        Ok(())
    }

    #[test]
    fn deserialize_text_only_request() {
        let input = serde_json::json!({
            "model": "test-model",
            "messages": [
                {"role": "user", "content": "hello"}
            ]
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        assert_eq!(params.model, "test-model");
        assert_eq!(params.messages.len(), 1);
        assert_eq!(params.messages[0].role, "user");
        assert_eq!(params.messages[0].content.text_content(), "hello");
    }

    #[test]
    fn deserialize_request_with_stream_options_include_usage_true() {
        let input = serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true,
            "stream_options": {"include_usage": true}
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        let stream_options = params.stream_options.unwrap();

        assert!(stream_options.include_usage);
    }

    #[test]
    fn deserialize_request_without_stream_options_defaults_to_none() {
        let input = serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        assert!(params.stream_options.is_none());
    }

    #[test]
    fn deserialize_multimodal_request_with_image() {
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

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        assert_eq!(params.messages.len(), 1);
        assert_eq!(
            params.messages[0].content.text_content(),
            "describe this image"
        );

        let image_urls = params.messages[0].content.image_urls();

        assert_eq!(image_urls.len(), 1);
        assert_eq!(image_urls[0].url, "data:image/jpeg;base64,/9j/4AAQ");
    }

    #[test]
    fn deserialize_multi_turn_conversation() {
        let input = serde_json::json!({
            "model": "test-model",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is 2+2?"},
                {"role": "assistant", "content": "4"},
                {"role": "user", "content": "And 3+3?"}
            ]
        });

        let params: super::OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        assert_eq!(params.messages.len(), 4);
    }

    #[test]
    fn openai_message_converts_to_conversation_message() {
        let input = serde_json::json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "OCR this"},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc"}}
            ]
        });

        let openai_message: super::OpenAIMessage = serde_json::from_value(input).unwrap();
        let conversation_message = openai_message.to_conversation_message();

        assert_eq!(conversation_message.role, "user");
        assert_eq!(conversation_message.content.text_content(), "OCR this");
        assert_eq!(conversation_message.content.image_urls().len(), 1);
    }

    fn invalid_json_call() -> ParsedToolCall {
        ParsedToolCall::new(
            "call_invalid".to_owned(),
            "broken_tool".to_owned(),
            ToolCallArguments::InvalidJson("{not valid json".to_owned()),
        )
    }

    fn assert_chunk_body_contains(result: &TransformResult, expected: &str) {
        let TransformResult::Chunk(content) = result else {
            panic!("expected a chunk variant");
        };

        assert!(
            content.contains(expected),
            "chunk does not contain '{expected}': {content}"
        );
    }

    fn assert_error_body_contains(result: &TransformResult, expected: &str) {
        let TransformResult::Error(content) = result else {
            panic!("expected an error variant");
        };

        assert!(
            content.contains(expected),
            "error does not contain '{expected}': {content}"
        );
    }

    #[actix_web::test]
    async fn streaming_tool_call_with_invalid_json_arguments_passes_raw_string_through() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallParsed(
                vec![invalid_json_call()],
            )))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_chunk_body_contains(&chunks[0], "{not valid json");
        assert_chunk_body_contains(&chunks[0], "\"name\":\"broken_tool\"");
    }

    #[actix_web::test]
    async fn non_streaming_tool_call_with_invalid_json_arguments_passes_raw_string_through() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallParsed(
                vec![invalid_json_call()],
            )))
            .await
            .unwrap();

        let final_chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::Done(
                summary_with_counts(3, 0, 0),
            )))
            .await
            .unwrap();

        assert_eq!(final_chunks.len(), 1);
        assert_chunk_body_contains(&final_chunks[0], "{not valid json");
        assert_chunk_body_contains(&final_chunks[0], "\"name\":\"broken_tool\"");
    }

    #[actix_web::test]
    async fn streaming_empty_parsed_tool_calls_emit_no_chunks() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::ToolCallParsed(
                Vec::new(),
            )))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 0);
    }

    #[actix_web::test]
    async fn streaming_embedding_response_returns_invalid_request_error() {
        let transformer = streaming_transformer(false);

        let message = make_response_message(OutgoingResponse::Embedding(
            paddler_messaging::embedding_result::EmbeddingResult::Done,
        ));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "invalid_request_error");
        assert_error_body_contains(
            &chunks[0],
            "unexpected embedding response in chat completions",
        );
    }

    #[actix_web::test]
    async fn non_streaming_embedding_response_returns_invalid_request_error() {
        let transformer = non_streaming_transformer();

        let message = make_response_message(OutgoingResponse::Embedding(
            paddler_messaging::embedding_result::EmbeddingResult::Done,
        ));
        let chunks = transformer.transform(message).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "invalid_request_error");
        assert_error_body_contains(
            &chunks[0],
            "unexpected embedding response in chat completions",
        );
    }

    #[actix_web::test]
    async fn streaming_grammar_incompatible_with_thinking_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::GrammarIncompatibleWithThinking(
                    "grammar conflicts with thinking".to_owned(),
                ),
            ))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "grammar conflicts with thinking");
        assert_error_body_contains(&chunks[0], "server_error");
    }

    #[actix_web::test]
    async fn streaming_grammar_rejected_model_output_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::GrammarRejectedModelOutput(
                    "output rejected by grammar".to_owned(),
                ),
            ))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "output rejected by grammar");
    }

    #[actix_web::test]
    async fn streaming_grammar_initialization_failed_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::GrammarInitializationFailed(
                    "could not build grammar".to_owned(),
                ),
            ))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "could not build grammar");
    }

    #[actix_web::test]
    async fn streaming_grammar_syntax_error_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(
                GeneratedTokenResult::GrammarSyntaxError("bad grammar syntax".to_owned()),
            ))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "bad grammar syntax");
    }

    #[actix_web::test]
    async fn streaming_sampler_error_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::SamplerError(
                "sampler blew up".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "sampler blew up");
    }

    #[actix_web::test]
    async fn streaming_tool_schema_invalid_returns_server_error() {
        let transformer = streaming_transformer(false);

        let chunks = transformer
            .transform(make_token_message(GeneratedTokenResult::ToolSchemaInvalid(
                "schema is not valid".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(chunks.len(), 1);
        assert_error_body_contains(&chunks[0], "schema is not valid");
    }

    fn app_data_without_agents(max_buffered_requests: i32) -> AppData {
        AppData {
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::ZERO,
                max_buffered_requests,
            )),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                cors_allowed_hosts: Vec::new(),
                inference_item_timeout: Duration::ZERO,
            },
            shutdown: CancellationToken::new(),
        }
    }

    #[actix_web::test]
    async fn invalid_tool_schema_returns_bad_request() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(register),
        )
        .await;

        let request = TestRequest::post()
            .uri("/v1/chat/completions")
            .set_json(serde_json::json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}],
                "tools": [
                    {
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
                    }
                ]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = read_body(response).await;
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(parsed["error"]["type"], "invalid_request_error");
        assert!(
            parsed["error"]["message"]
                .as_str()
                .unwrap()
                .contains("absent")
        );
    }

    #[actix_web::test]
    async fn non_streaming_request_without_available_agent_returns_internal_server_error() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(register),
        )
        .await;

        let request = TestRequest::post()
            .uri("/v1/chat/completions")
            .set_json(serde_json::json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = read_body(response).await;
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(parsed["error"]["type"], "server_error");
        assert!(
            parsed["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Buffered requests overflow")
        );
    }

    #[test]
    fn assert_chunk_contains_rejects_error_variant() {
        let outcome = assert_chunk_contains(&TransformResult::Error("boom".to_owned()), "boom");

        assert!(outcome.is_err());
        assert!(
            outcome
                .err()
                .unwrap()
                .to_string()
                .contains("expected TransformResult::Chunk")
        );
    }

    #[test]
    fn assert_chunk_does_not_contain_rejects_error_variant() {
        let outcome =
            assert_chunk_does_not_contain(&TransformResult::Error("boom".to_owned()), "boom");

        assert!(outcome.is_err());
        assert!(
            outcome
                .err()
                .unwrap()
                .to_string()
                .contains("expected TransformResult::Chunk")
        );
    }

    #[test]
    fn assert_error_contains_rejects_chunk_variant() {
        let outcome = assert_error_contains(&TransformResult::Chunk("body".to_owned()), "body");

        assert!(outcome.is_err());
        assert!(
            outcome
                .err()
                .unwrap()
                .to_string()
                .contains("expected TransformResult::Error")
        );
    }

    #[test]
    #[should_panic(expected = "expected a chunk variant")]
    fn assert_chunk_body_contains_panics_on_error_variant() {
        assert_chunk_body_contains(&TransformResult::Error("boom".to_owned()), "boom");
    }

    #[test]
    #[should_panic(expected = "expected an error variant")]
    fn assert_error_body_contains_panics_on_chunk_variant() {
        assert_error_body_contains(&TransformResult::Chunk("body".to_owned()), "body");
    }
}
