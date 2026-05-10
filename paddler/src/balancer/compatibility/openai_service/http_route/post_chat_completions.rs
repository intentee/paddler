use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use anyhow::Context as _;
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
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::TokenUsage;
use llama_cpp_bindings::ToolCallArguments;
use paddler_types::raw_tool_call_tokens::RawToolCallTokens;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
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

fn validation_failure_message(errors: &[String]) -> String {
    errors
        .first()
        .cloned()
        .unwrap_or_else(|| "tool call failed validation".to_owned())
}

fn unrecognized_tool_call_format_message(raw: &RawToolCallTokens) -> String {
    format!(
        "model produced output the parser did not recognise as any registered tool-call format; \
         FFI error: {}; raw text: {}",
        raw.ffi_error_message, raw.text,
    )
}

fn arguments_to_openai_string(arguments: &ToolCallArguments) -> Result<String> {
    match arguments {
        ToolCallArguments::ValidJson(value) => {
            serde_json::to_string(value).context("serializing tool-call arguments to OpenAI string")
        }
        ToolCallArguments::InvalidJson(raw) => Ok(raw.clone()),
    }
}

fn server_error_chunk(description: &str) -> TransformResult {
    TransformResult::Error(openai_error_json("server_error", description).to_string())
}

fn timeout_response_chunk() -> TransformResult {
    TransformResult::Error(openai_error_json("timeout", "request timed out").to_string())
}

fn rate_limit_response_chunk() -> TransformResult {
    TransformResult::Error(
        openai_error_json("rate_limit_error", "too many buffered requests").to_string(),
    )
}

fn unexpected_embedding_response_chunk() -> TransformResult {
    TransformResult::Error(
        openai_error_json(
            "invalid_request_error",
            "unexpected embedding response in chat completions",
        )
        .to_string(),
    )
}

fn description_from_error_token(token: &GeneratedTokenResult) -> Option<&str> {
    match token {
        GeneratedTokenResult::ChatTemplateError(description)
        | GeneratedTokenResult::GrammarIncompatibleWithThinking(description)
        | GeneratedTokenResult::GrammarRejectedModelOutput(description)
        | GeneratedTokenResult::GrammarInitializationFailed(description)
        | GeneratedTokenResult::GrammarSyntaxError(description)
        | GeneratedTokenResult::ImageDecodingFailed(description)
        | GeneratedTokenResult::MultimodalNotSupported(description)
        | GeneratedTokenResult::SamplerError(description)
        | GeneratedTokenResult::ToolCallParseFailed(description)
        | GeneratedTokenResult::ToolSchemaInvalid(description) => Some(description),
        _ => None,
    }
}

fn try_universal_error_chunk(message: &OutgoingMessage) -> Option<TransformResult> {
    match message {
        OutgoingMessage::Error(ErrorEnvelope {
            error: paddler_types::jsonrpc::Error { description, .. },
            ..
        }) => Some(server_error_chunk(description)),
        OutgoingMessage::Response(ResponseEnvelope { response, .. }) => match response {
            OutgoingResponse::GeneratedToken(token) => {
                description_from_error_token(token).map(server_error_chunk)
            }
            OutgoingResponse::Timeout => Some(timeout_response_chunk()),
            OutgoingResponse::TooManyBufferedRequests => Some(rate_limit_response_chunk()),
            OutgoingResponse::Embedding(_) => Some(unexpected_embedding_response_chunk()),
        },
    }
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

#[derive(Default)]
struct OpenAIStreamingState {
    saw_tool_call: bool,
}

#[derive(Clone)]
struct OpenAIStreamingResponseTransformer {
    include_usage: bool,
    model: String,
    state: Arc<Mutex<OpenAIStreamingState>>,
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

    fn tool_calls_chunk(
        &self,
        request_id: &str,
        parsed_calls: &[ParsedToolCall],
    ) -> Result<String> {
        let tool_calls = parsed_calls
            .iter()
            .enumerate()
            .map(|(index, call)| -> Result<serde_json::Value> {
                let arguments = arguments_to_openai_string(&call.arguments)?;
                Ok(json!({
                    "index": index,
                    "id": call.id,
                    "type": "function",
                    "function": {
                        "name": call.name,
                        "arguments": arguments,
                    }
                }))
            })
            .collect::<Result<Vec<_>>>()?;

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
                        "tool_calls": tool_calls,
                    },
                    "logprobs": null,
                    "finish_reason": null
                }
            ]
        }))?)
    }

    fn finish_chunk(&self, request_id: &str, finish_reason: &str) -> Result<String> {
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
                    "finish_reason": finish_reason
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

    fn handle_content(&self, request_id: &str, text: &str) -> Result<Vec<TransformResult>> {
        Ok(vec![TransformResult::Chunk(
            self.content_chunk(request_id, text)?,
        )])
    }

    fn handle_reasoning(&self, request_id: &str, text: &str) -> Result<Vec<TransformResult>> {
        Ok(vec![TransformResult::Chunk(
            self.reasoning_chunk(request_id, text)?,
        )])
    }

    fn handle_tool_call_parsed(
        &self,
        request_id: &str,
        parsed_calls: &[ParsedToolCall],
    ) -> Result<Vec<TransformResult>> {
        if parsed_calls.is_empty() {
            return Ok(vec![]);
        }

        self.state
            .lock()
            .map_err(|err| anyhow!("streaming state mutex poisoned: {err}"))?
            .saw_tool_call = true;

        Ok(vec![TransformResult::Chunk(
            self.tool_calls_chunk(request_id, parsed_calls)?,
        )])
    }

    fn handle_done(
        &self,
        request_id: &str,
        summary: &GenerationSummary,
    ) -> Result<Vec<TransformResult>> {
        let saw_tool_call = self
            .state
            .lock()
            .map_err(|err| anyhow!("streaming state mutex poisoned: {err}"))?
            .saw_tool_call;

        let finish_reason = if saw_tool_call { "tool_calls" } else { "stop" };
        let finish = TransformResult::Chunk(self.finish_chunk(request_id, finish_reason)?);

        if self.include_usage {
            let usage = TransformResult::Chunk(self.usage_chunk(request_id, &summary.usage)?);
            Ok(vec![finish, usage])
        } else {
            Ok(vec![finish])
        }
    }
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAIStreamingResponseTransformer {
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
                request_id,
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ReasoningToken(text)),
                ..
            }) => self.handle_reasoning(&request_id, &text),
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallToken(_)),
                ..
            }) => Ok(vec![]),
            OutgoingMessage::Response(ResponseEnvelope {
                request_id,
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallParsed(parsed_calls)),
                ..
            }) => self.handle_tool_call_parsed(&request_id, &parsed_calls),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallValidationFailed(
                        errors,
                    )),
                ..
            }) => Ok(vec![server_error_chunk(&validation_failure_message(
                &errors,
            ))]),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::UnrecognizedToolCallFormat(
                        raw,
                    )),
                ..
            }) => Ok(vec![server_error_chunk(
                &unrecognized_tool_call_format_message(&raw),
            )]),
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
    reasoning: String,
    tool_calls: Vec<ParsedToolCall>,
}

#[derive(Clone)]
struct OpenAINonStreamingResponseTransformer {
    model: String,
    state: Arc<Mutex<OpenAINonStreamingState>>,
}

impl OpenAINonStreamingResponseTransformer {
    fn append_content(&self, text: &str) -> Result<()> {
        self.state
            .lock()
            .map_err(|err| anyhow!("non-streaming state mutex poisoned: {err}"))?
            .content
            .push_str(text);
        Ok(())
    }

    fn append_reasoning(&self, text: &str) -> Result<()> {
        self.state
            .lock()
            .map_err(|err| anyhow!("non-streaming state mutex poisoned: {err}"))?
            .reasoning
            .push_str(text);
        Ok(())
    }

    fn append_tool_calls(&self, parsed_calls: Vec<ParsedToolCall>) -> Result<()> {
        self.state
            .lock()
            .map_err(|err| anyhow!("non-streaming state mutex poisoned: {err}"))?
            .tool_calls
            .extend(parsed_calls);
        Ok(())
    }

    fn build_done_chunk(&self, request_id: &str, summary: &GenerationSummary) -> Result<String> {
        let snapshot = self.snapshot_state()?;

        let has_tool_calls = !snapshot.tool_calls.is_empty();
        let finish_reason = if has_tool_calls { "tool_calls" } else { "stop" };

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

        if !snapshot.reasoning.is_empty()
            && let Some(map) = message_obj.as_object_mut()
        {
            map.insert("reasoning_content".to_owned(), json!(snapshot.reasoning));
        }

        if has_tool_calls && let Some(map) = message_obj.as_object_mut() {
            let tool_calls_json = snapshot
                .tool_calls
                .iter()
                .map(|call| -> Result<serde_json::Value> {
                    let arguments = arguments_to_openai_string(&call.arguments)?;
                    Ok(json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": arguments,
                        }
                    }))
                })
                .collect::<Result<Vec<_>>>()?;
            map.insert("tool_calls".to_owned(), json!(tool_calls_json));
        }

        Ok(serde_json::to_string(&json!({
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
        }))?)
    }

    fn snapshot_state(&self) -> Result<OpenAINonStreamingState> {
        let state = self
            .state
            .lock()
            .map_err(|err| anyhow!("non-streaming state mutex poisoned: {err}"))?;
        Ok(state.clone())
    }
}

#[async_trait]
impl TransformsOutgoingMessage for OpenAINonStreamingResponseTransformer {
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
                self.append_content(&text)?;
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ReasoningToken(text)),
                ..
            }) => {
                self.append_reasoning(&text)?;
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallToken(_)),
                ..
            }) => Ok(vec![]),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallParsed(parsed_calls)),
                ..
            }) => {
                self.append_tool_calls(parsed_calls)?;
                Ok(vec![])
            }
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::ToolCallValidationFailed(
                        errors,
                    )),
                ..
            }) => Ok(vec![server_error_chunk(&validation_failure_message(
                &errors,
            ))]),
            OutgoingMessage::Response(ResponseEnvelope {
                response:
                    OutgoingResponse::GeneratedToken(GeneratedTokenResult::UnrecognizedToolCallFormat(
                        raw,
                    )),
                ..
            }) => Ok(vec![server_error_chunk(
                &unrecognized_tool_call_format_message(&raw),
            )]),
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
                .body(openai_error_json("invalid_request_error", &err.to_string()).to_string()));
        }
    };

    let parse_tool_calls = !validated_tools.is_empty();
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
        parse_tool_calls,
        tools: validated_tools,
    };

    if openai_params.stream.unwrap_or(false) {
        let include_usage = openai_params
            .stream_options
            .as_ref()
            .is_some_and(|options| options.include_usage);

        Ok(http_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAIStreamingResponseTransformer {
                include_usage,
                model: openai_params.model.clone(),
                state: Arc::new(Mutex::new(OpenAIStreamingState::default())),
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

        let body = results.into_iter().find_map(|result| match result {
            TransformResult::Chunk(content) => Some(content),
            TransformResult::Discard | TransformResult::Error(_) => None,
        });

        Ok(body.map_or_else(
            || {
                HttpResponse::InternalServerError()
                    .content_type("application/json")
                    .body(openai_error_json("server_error", "no completion produced").to_string())
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
    use std::sync::Arc;
    use std::sync::Mutex;

    use anyhow::Result;
    use llama_cpp_bindings::ParsedToolCall;
    use llama_cpp_bindings::TokenUsage;
    use llama_cpp_bindings::ToolCallArguments;
    use paddler_types::generated_token_result::GeneratedTokenResult;
    use paddler_types::generation_summary::GenerationSummary;
    use paddler_types::inference_client::Message as OutgoingMessage;
    use paddler_types::inference_client::Response as OutgoingResponse;
    use paddler_types::jsonrpc::ErrorEnvelope;
    use paddler_types::jsonrpc::ResponseEnvelope;

    use super::OpenAINonStreamingResponseTransformer;
    use super::OpenAINonStreamingState;
    use super::OpenAIStreamingResponseTransformer;
    use crate::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::balancer::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;

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
            error: paddler_types::jsonrpc::Error {
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
            include_usage,
            model: "test-model".to_owned(),
            state: Arc::new(Mutex::new(super::OpenAIStreamingState::default())),
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
    async fn streaming_reasoning_token_emits_reasoning_content_delta() -> Result<()> {
        let transformer = streaming_transformer(false);

        let message =
            make_token_message(GeneratedTokenResult::ReasoningToken("thought".to_owned()));
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
                    paddler_types::raw_tool_call_tokens::RawToolCallTokens {
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
                    paddler_types::raw_tool_call_tokens::RawToolCallTokens {
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
    fn openai_error_json_has_correct_structure() {
        let error = super::openai_error_json("server_error", "something went wrong");

        assert_eq!(error["error"]["type"], "server_error");
        assert_eq!(error["error"]["message"], "something went wrong");
        assert!(error["error"]["param"].is_null());
        assert!(error["error"]["code"].is_null());
    }

    #[test]
    fn validation_failure_message_returns_first_error() {
        let message = super::validation_failure_message(&[
            "first issue".to_owned(),
            "second issue".to_owned(),
        ]);

        assert_eq!(message, "first issue");
    }

    #[test]
    fn validation_failure_message_falls_back_when_no_errors() {
        let message = super::validation_failure_message(&[]);

        assert!(message.contains("validation"));
    }
}
