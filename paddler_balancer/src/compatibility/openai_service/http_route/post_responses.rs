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
use llama_cpp_bindings_types::ParsedToolCall;
use llama_cpp_bindings_types::TokenUsage;
use nanoid::nanoid;
use parking_lot::Mutex;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::validates::Validates;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use tokio_stream::StreamExt as _;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::responses_stream_event::ContentPartEvent;
use crate::compatibility::openai_service::responses_stream_event::FunctionCallArgumentsDeltaEvent;
use crate::compatibility::openai_service::responses_stream_event::FunctionCallArgumentsDoneEvent;
use crate::compatibility::openai_service::responses_stream_event::OutputItemEvent;
use crate::compatibility::openai_service::responses_stream_event::ResponseSnapshotEvent;
use crate::compatibility::openai_service::responses_stream_event::ResponsesStreamEvent;
use crate::compatibility::openai_service::responses_stream_event::TextDeltaEvent;
use crate::compatibility::openai_service::responses_stream_event::TextDoneEvent;
use crate::compatibility::openai_service::sse_response_from_agent::sse_response_from_agent;
use crate::compatibility::openai_service::timestamp_from::timestamp_from;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;

const DEFAULT_MAX_TOKENS: i32 = 2000;

fn responses_error(message: &OutgoingMessage) -> Option<OpenAIError> {
    if let OutgoingMessage::Response(ResponseEnvelope {
        response: OutgoingResponse::Embedding(_),
        ..
    }) = message
    {
        return Some(OpenAIError {
            error_type: "invalid_request_error",
            message: "unexpected embedding response in responses".to_owned(),
        });
    }

    OpenAIError::classify(message)
}

fn responses_usage_json(usage: &TokenUsage) -> Value {
    json!({
        "input_tokens": usage.prompt_tokens,
        "input_tokens_details": { "cached_tokens": usage.cached_prompt_tokens },
        "output_tokens": usage.completion_tokens(),
        "output_tokens_details": { "reasoning_tokens": usage.reasoning_tokens },
        "total_tokens": usage.total_tokens(),
    })
}

fn output_text_part(text: &str) -> Value {
    json!({
        "type": "output_text",
        "text": text,
        "annotations": [],
        "logprobs": []
    })
}

fn message_item_open(item_id: &str) -> Value {
    json!({
        "id": item_id,
        "type": "message",
        "role": "assistant",
        "status": "in_progress",
        "content": []
    })
}

fn message_item_done(item_id: &str, text: &str) -> Value {
    json!({
        "id": item_id,
        "type": "message",
        "role": "assistant",
        "status": "completed",
        "content": [output_text_part(text)]
    })
}

fn reasoning_item_open(item_id: &str) -> Value {
    json!({
        "type": "reasoning",
        "id": item_id,
        "summary": [],
        "status": "in_progress"
    })
}

fn reasoning_item_done(item_id: &str, text: &str) -> Value {
    json!({
        "type": "reasoning",
        "id": item_id,
        "summary": [],
        "content": [{ "type": "reasoning_text", "text": text }],
        "status": "completed"
    })
}

fn function_call_item(
    item_id: &str,
    call_id: &str,
    name: &str,
    arguments: &str,
    status: &str,
) -> Value {
    json!({
        "type": "function_call",
        "id": item_id,
        "call_id": call_id,
        "name": name,
        "arguments": arguments,
        "status": status
    })
}

fn normalize_role(role: String) -> String {
    if role == "developer" {
        "system".to_owned()
    } else {
        role
    }
}

fn input_content_part_to_conversation(
    part: OpenAIResponsesInputContentPart,
) -> Option<ConversationMessageContentPart> {
    match part {
        OpenAIResponsesInputContentPart::InputText { text } => {
            Some(ConversationMessageContentPart::Text { text })
        }
        OpenAIResponsesInputContentPart::InputImage {
            image_url: Some(url),
        } => Some(ConversationMessageContentPart::ImageUrl {
            image_url: ImageUrl { url },
        }),
        OpenAIResponsesInputContentPart::InputImage { image_url: None }
        | OpenAIResponsesInputContentPart::Unsupported => None,
    }
}

fn message_content_to_conversation(
    content: OpenAIResponsesMessageContent,
) -> ConversationMessageContent {
    match content {
        OpenAIResponsesMessageContent::Text(text) => ConversationMessageContent::Text(text),
        OpenAIResponsesMessageContent::Parts(parts) => ConversationMessageContent::Parts(
            parts
                .into_iter()
                .filter_map(input_content_part_to_conversation)
                .collect(),
        ),
    }
}

fn function_output_to_text(output: OpenAIResponsesFunctionOutput) -> String {
    match output {
        OpenAIResponsesFunctionOutput::Text(text) => text,
        OpenAIResponsesFunctionOutput::Parts(parts) => parts
            .into_iter()
            .filter_map(|part| match part {
                OpenAIResponsesInputContentPart::InputText { text } => Some(text),
                OpenAIResponsesInputContentPart::InputImage { .. }
                | OpenAIResponsesInputContentPart::Unsupported => None,
            })
            .collect::<String>(),
    }
}

fn message_item_to_conversation(
    OpenAIResponsesMessageItem { role, content }: OpenAIResponsesMessageItem,
) -> ConversationMessage {
    ConversationMessage {
        content: message_content_to_conversation(content),
        role: normalize_role(role),
    }
}

fn input_item_to_conversation(item: OpenAIResponsesInputItem) -> Option<ConversationMessage> {
    match item {
        OpenAIResponsesInputItem::Message(message)
        | OpenAIResponsesInputItem::Tagged(OpenAIResponsesTaggedItem::Message(message)) => {
            Some(message_item_to_conversation(message))
        }
        OpenAIResponsesInputItem::Tagged(OpenAIResponsesTaggedItem::FunctionCall(
            OpenAIResponsesFunctionCallItem {
                call_id,
                name,
                arguments,
            },
        )) => Some(ConversationMessage {
            content: ConversationMessageContent::Text(
                json!({ "call_id": call_id, "name": name, "arguments": arguments }).to_string(),
            ),
            role: "assistant".to_owned(),
        }),
        OpenAIResponsesInputItem::Tagged(OpenAIResponsesTaggedItem::FunctionCallOutput(
            OpenAIResponsesFunctionCallOutputItem { output },
        )) => Some(ConversationMessage {
            content: ConversationMessageContent::Text(function_output_to_text(output)),
            role: "tool".to_owned(),
        }),
        OpenAIResponsesInputItem::Tagged(OpenAIResponsesTaggedItem::Unsupported) => None,
    }
}

fn grammar_from_text(text: Option<OpenAIResponsesTextParam>) -> Result<Option<GrammarConstraint>> {
    match text.and_then(|text| text.format) {
        Some(OpenAIResponsesTextFormat::JsonSchema { schema }) => {
            Ok(Some(GrammarConstraint::JsonSchema {
                schema: serde_json::to_string(&schema)
                    .context("serializing responses text.format json schema")?,
            }))
        }
        Some(OpenAIResponsesTextFormat::Text | OpenAIResponsesTextFormat::Unsupported) | None => {
            Ok(None)
        }
    }
}

fn enable_thinking_from_reasoning(reasoning: Option<OpenAIResponsesReasoning>) -> bool {
    !matches!(
        reasoning.and_then(|reasoning| reasoning.effort).as_deref(),
        Some("none")
    )
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OpenAIResponsesMessageContent {
    Text(String),
    Parts(Vec<OpenAIResponsesInputContentPart>),
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum OpenAIResponsesInputContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage {
        #[serde(default)]
        image_url: Option<String>,
    },
    #[serde(other)]
    Unsupported,
}

#[derive(Deserialize)]
struct OpenAIResponsesMessageItem {
    role: String,
    content: OpenAIResponsesMessageContent,
}

#[derive(Deserialize)]
struct OpenAIResponsesFunctionCallItem {
    call_id: String,
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OpenAIResponsesFunctionOutput {
    Text(String),
    Parts(Vec<OpenAIResponsesInputContentPart>),
}

#[derive(Deserialize)]
struct OpenAIResponsesFunctionCallOutputItem {
    output: OpenAIResponsesFunctionOutput,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum OpenAIResponsesTaggedItem {
    #[serde(rename = "message")]
    Message(OpenAIResponsesMessageItem),
    #[serde(rename = "function_call")]
    FunctionCall(OpenAIResponsesFunctionCallItem),
    #[serde(rename = "function_call_output")]
    FunctionCallOutput(OpenAIResponsesFunctionCallOutputItem),
    #[serde(other)]
    Unsupported,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OpenAIResponsesInputItem {
    Tagged(OpenAIResponsesTaggedItem),
    Message(OpenAIResponsesMessageItem),
}

enum OpenAIResponsesInput {
    Text(String),
    Items(Vec<OpenAIResponsesInputItem>),
}

impl Default for OpenAIResponsesInput {
    fn default() -> Self {
        Self::Items(Vec::new())
    }
}

impl<'de> Deserialize<'de> for OpenAIResponsesInput {
    fn deserialize<TDeserializer>(deserializer: TDeserializer) -> Result<Self, TDeserializer::Error>
    where
        TDeserializer: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TextOrItems {
            Text(String),
            Items(Vec<OpenAIResponsesInputItem>),
        }

        Ok(match TextOrItems::deserialize(deserializer)? {
            TextOrItems::Text(text) => Self::Text(text),
            TextOrItems::Items(items) => Self::Items(items),
        })
    }
}

#[derive(Deserialize)]
struct OpenAIResponsesFunctionTool {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parameters: Option<RawParametersSchema>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum OpenAIResponsesTool {
    // Boxed because the function-tool payload is far larger than the empty `Unsupported` variant.
    #[serde(rename = "function")]
    Function(Box<OpenAIResponsesFunctionTool>),
    #[serde(other)]
    Unsupported,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum OpenAIResponsesTextFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_schema")]
    JsonSchema { schema: Value },
    #[serde(other)]
    Unsupported,
}

#[derive(Deserialize)]
struct OpenAIResponsesTextParam {
    #[serde(default)]
    format: Option<OpenAIResponsesTextFormat>,
}

#[derive(Deserialize)]
struct OpenAIResponsesReasoning {
    #[serde(default)]
    effort: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIResponsesRequestParams {
    /// Echoed back in the response object; not used for routing.
    model: String,
    #[serde(default)]
    input: OpenAIResponsesInput,
    #[serde(default)]
    instructions: Option<String>,
    #[serde(default)]
    stream: Option<bool>,
    #[serde(default)]
    max_output_tokens: Option<i32>,
    #[serde(default)]
    tools: Vec<OpenAIResponsesTool>,
    #[serde(default)]
    text: Option<OpenAIResponsesTextParam>,
    #[serde(default)]
    reasoning: Option<OpenAIResponsesReasoning>,
}

struct ResponsesPreparedRequest {
    paddler_params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    stream: bool,
    model: String,
    instructions: Option<String>,
}

impl OpenAIResponsesRequestParams {
    fn into_prepared(self) -> Result<ResponsesPreparedRequest> {
        let Self {
            model,
            input,
            instructions,
            stream,
            max_output_tokens,
            tools,
            text,
            reasoning,
        } = self;

        let mut messages: Vec<ConversationMessage> = Vec::new();

        if let Some(instructions) = &instructions
            && !instructions.is_empty()
        {
            messages.push(ConversationMessage {
                content: ConversationMessageContent::Text(instructions.clone()),
                role: "system".to_owned(),
            });
        }

        match input {
            OpenAIResponsesInput::Text(text) => messages.push(ConversationMessage {
                content: ConversationMessageContent::Text(text),
                role: "user".to_owned(),
            }),
            OpenAIResponsesInput::Items(items) => {
                messages.extend(items.into_iter().filter_map(input_item_to_conversation));
            }
        }

        let validated_tools = tools
            .into_iter()
            .filter_map(|tool| match tool {
                OpenAIResponsesTool::Function(function_tool) => {
                    let OpenAIResponsesFunctionTool {
                        name,
                        description,
                        parameters,
                    } = *function_tool;

                    Some(Tool::Function(FunctionCall {
                        function: Function {
                            name,
                            description: description.unwrap_or_default(),
                            parameters: parameters.map_or(Parameters::Empty, Parameters::Schema),
                        },
                    }))
                }
                OpenAIResponsesTool::Unsupported => None,
            })
            .map(Validates::validate)
            .collect::<Result<Vec<_>>>()?;

        let parse_tool_calls = !validated_tools.is_empty();

        Ok(ResponsesPreparedRequest {
            paddler_params: ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(messages),
                enable_thinking: enable_thinking_from_reasoning(reasoning),
                grammar: grammar_from_text(text)?,
                max_tokens: max_output_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
                parse_tool_calls,
                tools: validated_tools,
            },
            stream: stream.unwrap_or(false),
            model,
            instructions,
        })
    }
}

#[derive(Clone)]
struct ResponsesResponseBuilder {
    id: String,
    created_at: u64,
    model: String,
    instructions: Option<String>,
}

impl ResponsesResponseBuilder {
    // `usage` is intentionally absent here: the official `ResponseUsage` reference is not nullable, so the
    // in-progress and failed snapshots must omit it rather than emit `null`. Only `completed` adds it.
    fn base(&self, status: &str, output: &Value, error: &Value) -> Value {
        let instructions = self
            .instructions
            .as_ref()
            .map_or(Value::Null, |instructions| json!(instructions));

        json!({
            "id": self.id,
            "object": "response",
            "created_at": self.created_at,
            "status": status,
            "error": error,
            "incomplete_details": null,
            "instructions": instructions,
            "model": self.model,
            "tools": [],
            "output": output,
            "parallel_tool_calls": true,
            "metadata": {},
            "tool_choice": "auto",
            "temperature": 1,
            "top_p": 1,
            "text": { "format": { "type": "text" } }
        })
    }

    fn in_progress(&self) -> Value {
        self.base("in_progress", &json!([]), &Value::Null)
    }

    fn completed(&self, output: Vec<Value>, usage: &TokenUsage) -> Value {
        let mut response = self.base("completed", &Value::Array(output), &Value::Null);

        if let Some(object) = response.as_object_mut() {
            object.insert("usage".to_owned(), responses_usage_json(usage));
        }

        response
    }

    fn failed(&self, error: &OpenAIError) -> Value {
        self.base(
            "failed",
            &json!([]),
            &json!({ "code": "server_error", "message": error.message }),
        )
    }
}

#[derive(Default, PartialEq)]
enum OpenItem {
    #[default]
    None,
    Reasoning,
    Message,
}

#[derive(Default)]
struct ResponsesStreamingState {
    started: bool,
    sequence_number: u64,
    output_index: usize,
    open: OpenItem,
    reasoning_id: String,
    reasoning_text: String,
    message_id: String,
    message_text: String,
    finalized_output: Vec<Value>,
}

impl ResponsesStreamingState {
    const fn next_sequence_number(&mut self) -> u64 {
        let sequence_number = self.sequence_number;
        self.sequence_number += 1;

        sequence_number
    }

    fn close_open_item(&mut self, events: &mut Vec<ResponsesStreamEvent>) {
        match self.open {
            OpenItem::None => {}
            OpenItem::Reasoning => {
                let item_id = self.reasoning_id.clone();
                let text = self.reasoning_text.clone();
                let output_index = self.output_index;

                let text_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::ReasoningTextDone(TextDoneEvent {
                    sequence_number: text_done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    text: text.clone(),
                }));

                let item = reasoning_item_done(&item_id, &text);
                let item_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::OutputItemDone(OutputItemEvent {
                    sequence_number: item_done_sequence_number,
                    output_index,
                    item: item.clone(),
                }));

                self.finalized_output.push(item);
                self.output_index += 1;
                self.reasoning_text.clear();
                self.open = OpenItem::None;
            }
            OpenItem::Message => {
                let item_id = self.message_id.clone();
                let text = self.message_text.clone();
                let output_index = self.output_index;

                let text_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::OutputTextDone(TextDoneEvent {
                    sequence_number: text_done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    text: text.clone(),
                }));

                let part_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::ContentPartDone(ContentPartEvent {
                    sequence_number: part_done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    part: output_text_part(&text),
                }));

                let item = message_item_done(&item_id, &text);
                let item_done_sequence_number = self.next_sequence_number();
                events.push(ResponsesStreamEvent::OutputItemDone(OutputItemEvent {
                    sequence_number: item_done_sequence_number,
                    output_index,
                    item: item.clone(),
                }));

                self.finalized_output.push(item);
                self.output_index += 1;
                self.message_text.clear();
                self.open = OpenItem::None;
            }
        }
    }

    fn handle_reasoning(&mut self, events: &mut Vec<ResponsesStreamEvent>, text: &str) {
        if self.open != OpenItem::Reasoning {
            self.close_open_item(events);

            let output_index = self.output_index;
            let item_id = format!("rs_{output_index}");
            self.reasoning_id.clone_from(&item_id);

            let added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::OutputItemAdded(OutputItemEvent {
                sequence_number: added_sequence_number,
                output_index,
                item: reasoning_item_open(&item_id),
            }));

            self.open = OpenItem::Reasoning;
        }

        self.reasoning_text.push_str(text);

        let item_id = self.reasoning_id.clone();
        let output_index = self.output_index;
        let delta_sequence_number = self.next_sequence_number();
        events.push(ResponsesStreamEvent::ReasoningTextDelta(TextDeltaEvent {
            sequence_number: delta_sequence_number,
            item_id,
            output_index,
            content_index: 0,
            delta: text.to_owned(),
        }));
    }

    fn handle_content(&mut self, events: &mut Vec<ResponsesStreamEvent>, text: &str) {
        if self.open != OpenItem::Message {
            self.close_open_item(events);

            let output_index = self.output_index;
            let item_id = format!("msg_{output_index}");
            self.message_id.clone_from(&item_id);

            let added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::OutputItemAdded(OutputItemEvent {
                sequence_number: added_sequence_number,
                output_index,
                item: message_item_open(&item_id),
            }));

            let part_added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::ContentPartAdded(ContentPartEvent {
                sequence_number: part_added_sequence_number,
                item_id,
                output_index,
                content_index: 0,
                part: output_text_part(""),
            }));

            self.open = OpenItem::Message;
        }

        self.message_text.push_str(text);

        let item_id = self.message_id.clone();
        let output_index = self.output_index;
        let delta_sequence_number = self.next_sequence_number();
        events.push(ResponsesStreamEvent::OutputTextDelta(TextDeltaEvent {
            sequence_number: delta_sequence_number,
            item_id,
            output_index,
            content_index: 0,
            delta: text.to_owned(),
        }));
    }

    fn handle_tool_calls(
        &mut self,
        events: &mut Vec<ResponsesStreamEvent>,
        parsed_calls: &[ParsedToolCall],
    ) -> Result<()> {
        self.close_open_item(events);

        for call in parsed_calls {
            let output_index = self.output_index;
            let item_id = format!("fc_{output_index}");
            let arguments = arguments_to_tool_call_string(&call.arguments)?;

            let added_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::OutputItemAdded(OutputItemEvent {
                sequence_number: added_sequence_number,
                output_index,
                item: function_call_item(&item_id, &call.id, &call.name, "", "in_progress"),
            }));

            let delta_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::FunctionCallArgumentsDelta(
                FunctionCallArgumentsDeltaEvent {
                    sequence_number: delta_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    delta: arguments.clone(),
                },
            ));

            let done_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::FunctionCallArgumentsDone(
                FunctionCallArgumentsDoneEvent {
                    sequence_number: done_sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    name: call.name.clone(),
                    arguments: arguments.clone(),
                },
            ));

            let item = function_call_item(&item_id, &call.id, &call.name, &arguments, "completed");
            let item_done_sequence_number = self.next_sequence_number();
            events.push(ResponsesStreamEvent::OutputItemDone(OutputItemEvent {
                sequence_number: item_done_sequence_number,
                output_index,
                item: item.clone(),
            }));

            self.finalized_output.push(item);
            self.output_index += 1;
        }

        Ok(())
    }
}

#[derive(Clone)]
struct ResponsesStreamingResponseTransformer {
    builder: ResponsesResponseBuilder,
    state: Arc<Mutex<ResponsesStreamingState>>,
}

impl ResponsesStreamingResponseTransformer {
    fn ensure_preamble(
        &self,
        state: &mut ResponsesStreamingState,
        events: &mut Vec<ResponsesStreamEvent>,
    ) {
        if state.started {
            return;
        }

        state.started = true;

        let created_sequence_number = state.next_sequence_number();
        events.push(ResponsesStreamEvent::Created(ResponseSnapshotEvent {
            sequence_number: created_sequence_number,
            response: self.builder.in_progress(),
        }));

        let in_progress_sequence_number = state.next_sequence_number();
        events.push(ResponsesStreamEvent::InProgress(ResponseSnapshotEvent {
            sequence_number: in_progress_sequence_number,
            response: self.builder.in_progress(),
        }));
    }

    fn handle_done(
        &self,
        state: &mut ResponsesStreamingState,
        events: &mut Vec<ResponsesStreamEvent>,
        summary: &GenerationSummary,
    ) {
        state.close_open_item(events);

        let output = state.finalized_output.clone();
        let completed_sequence_number = state.next_sequence_number();
        events.push(ResponsesStreamEvent::Completed(ResponseSnapshotEvent {
            sequence_number: completed_sequence_number,
            response: self.builder.completed(output, &summary.usage),
        }));
    }
}

#[async_trait]
impl TransformsOutgoingMessage for ResponsesStreamingResponseTransformer {
    type Output = ResponsesStreamEvent;

    // The streaming state transition for a single message composes several mutations (lazy preamble,
    // opening/closing the current output item, deltas) that must observe one consistent lock; the helper
    // methods call each other and `parking_lot` is non-reentrant, so a single guard must span the whole
    // transition. `transform` is also invoked serially per request, so there is no concurrent access to
    // contend for.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "one guard must span the whole per-message state transition; calls are serial so there is no contention"
    )]
    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<ResponsesStreamEvent>> {
        let mut events: Vec<ResponsesStreamEvent> = Vec::new();
        let mut state = self.state.lock();

        if let Some(error) = responses_error(&message) {
            self.ensure_preamble(&mut state, &mut events);

            let failed_sequence_number = state.next_sequence_number();
            events.push(ResponsesStreamEvent::Failed(ResponseSnapshotEvent {
                sequence_number: failed_sequence_number,
                response: self.builder.failed(&error),
            }));

            return Ok(events);
        }

        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(token),
                ..
            }) => match token {
                GeneratedTokenResult::ContentToken(text)
                | GeneratedTokenResult::UndeterminableToken(text) => {
                    self.ensure_preamble(&mut state, &mut events);
                    state.handle_content(&mut events, &text);
                }
                GeneratedTokenResult::ReasoningToken(text) => {
                    self.ensure_preamble(&mut state, &mut events);
                    state.handle_reasoning(&mut events, &text);
                }
                GeneratedTokenResult::ToolCallToken(_) => {}
                GeneratedTokenResult::ToolCallParsed(parsed_calls) => {
                    self.ensure_preamble(&mut state, &mut events);
                    state.handle_tool_calls(&mut events, &parsed_calls)?;
                }
                GeneratedTokenResult::Done(summary) => {
                    self.ensure_preamble(&mut state, &mut events);
                    self.handle_done(&mut state, &mut events, &summary);
                }
                other => {
                    return Err(anyhow!(
                        "ResponsesStreamingResponseTransformer received a token it does not know how to handle: {other:?}"
                    ));
                }
            },
            other => {
                return Err(anyhow!(
                    "ResponsesStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
                ));
            }
        }

        Ok(events)
    }
}

#[derive(Clone, Default)]
struct ResponsesNonStreamingState {
    content: String,
    reasoning: String,
    tool_calls: Vec<ParsedToolCall>,
}

#[derive(Clone)]
struct ResponsesNonStreamingResponseTransformer {
    builder: ResponsesResponseBuilder,
    state: Arc<Mutex<ResponsesNonStreamingState>>,
}

impl ResponsesNonStreamingResponseTransformer {
    fn build_completed(&self, summary: &GenerationSummary) -> Result<String> {
        let snapshot = self.state.lock().clone();

        let mut output: Vec<Value> = Vec::new();

        if !snapshot.reasoning.is_empty() {
            output.push(reasoning_item_done(
                &format!("rs_{}", output.len()),
                &snapshot.reasoning,
            ));
        }

        let has_tool_calls = !snapshot.tool_calls.is_empty();

        if !snapshot.content.is_empty() || !has_tool_calls {
            output.push(message_item_done(
                &format!("msg_{}", output.len()),
                &snapshot.content,
            ));
        }

        for call in &snapshot.tool_calls {
            let arguments = arguments_to_tool_call_string(&call.arguments)?;

            output.push(function_call_item(
                &format!("fc_{}", output.len()),
                &call.id,
                &call.name,
                &arguments,
                "completed",
            ));
        }

        serde_json::to_string(&self.builder.completed(output, &summary.usage))
            .context("serializing non-streaming responses completion")
    }
}

#[async_trait]
impl TransformsOutgoingMessage for ResponsesNonStreamingResponseTransformer {
    type Output = TransformResult;

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        if let Some(error) = responses_error(&message) {
            return Ok(vec![TransformResult::Error(
                error.to_envelope().to_string(),
            )]);
        }

        match message {
            OutgoingMessage::Response(ResponseEnvelope {
                response: OutgoingResponse::GeneratedToken(token),
                ..
            }) => match token {
                GeneratedTokenResult::ContentToken(text)
                | GeneratedTokenResult::UndeterminableToken(text) => {
                    self.state.lock().content.push_str(&text);
                    Ok(vec![])
                }
                GeneratedTokenResult::ReasoningToken(text) => {
                    self.state.lock().reasoning.push_str(&text);
                    Ok(vec![])
                }
                GeneratedTokenResult::ToolCallToken(_) => Ok(vec![]),
                GeneratedTokenResult::ToolCallParsed(parsed_calls) => {
                    self.state.lock().tool_calls.extend(parsed_calls);
                    Ok(vec![])
                }
                GeneratedTokenResult::Done(summary) => Ok(vec![TransformResult::Chunk(
                    self.build_completed(&summary)?,
                )]),
                other => Err(anyhow!(
                    "ResponsesNonStreamingResponseTransformer received a token it does not know how to handle: {other:?}"
                )),
            },
            other => Err(anyhow!(
                "ResponsesNonStreamingResponseTransformer received an outgoing message it does not know how to handle: {other:?}"
            )),
        }
    }
}

#[post("/v1/responses")]
async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAIResponsesRequestParams>,
) -> Result<HttpResponse, Error> {
    let prepared = match openai_params.into_inner().into_prepared() {
        Ok(prepared) => prepared,
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

    let created_at =
        timestamp_from(SystemTime::now()).map_err(actix_web::error::ErrorInternalServerError)?;

    let builder = ResponsesResponseBuilder {
        id: format!("resp_{}", nanoid!()),
        created_at,
        model: prepared.model,
        instructions: prepared.instructions,
    };

    if prepared.stream {
        Ok(sse_response_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            prepared.paddler_params,
            ResponsesStreamingResponseTransformer {
                builder,
                state: Arc::new(Mutex::new(ResponsesStreamingState::default())),
            },
            app_data.shutdown.clone(),
        ))
    } else {
        let results: Vec<TransformResult> = unbounded_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            prepared.paddler_params,
            ResponsesNonStreamingResponseTransformer {
                builder,
                state: Arc::new(Mutex::new(ResponsesNonStreamingState::default())),
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
                            message: "no response produced".to_owned(),
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

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::grammar_constraint::GrammarConstraint;
    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::inference_client::response::Response as OutgoingResponse;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
    use parking_lot::Mutex;
    use serde_json::json;

    use super::OpenAIResponsesRequestParams;
    use super::ResponsesNonStreamingResponseTransformer;
    use super::ResponsesNonStreamingState;
    use super::ResponsesResponseBuilder;
    use super::ResponsesStreamingResponseTransformer;
    use super::ResponsesStreamingState;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
    use crate::compatibility::openai_service::responses_stream_event::ResponsesStreamEvent;

    fn builder() -> ResponsesResponseBuilder {
        ResponsesResponseBuilder {
            id: "resp_test".to_owned(),
            created_at: 0,
            model: "test-model".to_owned(),
            instructions: None,
        }
    }

    fn streaming_transformer() -> ResponsesStreamingResponseTransformer {
        ResponsesStreamingResponseTransformer {
            builder: builder(),
            state: Arc::new(Mutex::new(ResponsesStreamingState::default())),
        }
    }

    fn non_streaming_transformer() -> ResponsesNonStreamingResponseTransformer {
        ResponsesNonStreamingResponseTransformer {
            builder: builder(),
            state: Arc::new(Mutex::new(ResponsesNonStreamingState::default())),
        }
    }

    fn token_message(token_result: GeneratedTokenResult) -> OutgoingMessage {
        OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "test-request".to_owned(),
            response: OutgoingResponse::GeneratedToken(token_result),
        })
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
            ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
        )
    }

    fn names(events: &[ResponsesStreamEvent]) -> Vec<&'static str> {
        events
            .iter()
            .map(ResponsesStreamEvent::event_name)
            .collect()
    }

    #[tokio::test]
    async fn streaming_first_content_token_emits_preamble_then_text_delta() {
        let transformer = streaming_transformer();

        let events = transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hi".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.output_item.added",
                "response.content_part.added",
                "response.output_text.delta",
            ]
        );
        assert_eq!(events[0].to_json()["response"]["status"], "in_progress");
        assert_eq!(events[4].to_json()["delta"], "hi");
    }

    #[tokio::test]
    async fn streaming_preamble_is_emitted_only_once() {
        let transformer = streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "a".to_owned(),
            )))
            .await
            .unwrap();
        let events = transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "b".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(names(&events), vec!["response.output_text.delta"]);
    }

    #[tokio::test]
    async fn streaming_done_finalizes_message_and_emits_completed_with_usage() {
        let transformer = streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hello".to_owned(),
            )))
            .await
            .unwrap();
        let events = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(7, 4, 1),
            )))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.output_text.done",
                "response.content_part.done",
                "response.output_item.done",
                "response.completed",
            ]
        );

        let completed = events[3].to_json();

        assert_eq!(completed["response"]["status"], "completed");
        assert_eq!(completed["response"]["usage"]["input_tokens"], 7);
        assert_eq!(completed["response"]["usage"]["total_tokens"], 12);
        assert_eq!(
            completed["response"]["output"][0]["content"][0]["text"],
            "hello"
        );
        assert_eq!(
            completed["response"]["output"][0]["content"][0]["logprobs"],
            json!([])
        );
    }

    #[tokio::test]
    async fn streaming_reasoning_then_content_closes_the_reasoning_item_first() {
        let transformer = streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ReasoningToken(
                "think".to_owned(),
            )))
            .await
            .unwrap();
        let events = transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "answer".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.reasoning_text.done",
                "response.output_item.done",
                "response.output_item.added",
                "response.content_part.added",
                "response.output_text.delta",
            ]
        );
        // reasoning item closed at output_index 0, message opened at output_index 1
        assert_eq!(events[1].to_json()["output_index"], 0);
        assert_eq!(events[2].to_json()["output_index"], 1);
        assert_eq!(events[1].to_json()["item"]["type"], "reasoning");
    }

    #[tokio::test]
    async fn streaming_tool_call_emits_function_call_argument_events_without_content_index() {
        let transformer = streaming_transformer();

        let events = transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.output_item.added",
                "response.function_call_arguments.delta",
                "response.function_call_arguments.done",
                "response.output_item.done",
            ]
        );

        let delta_event = events[3].to_json();

        assert_eq!(delta_event["delta"], "{\"location\":\"Paris\"}");
        assert!(
            delta_event.get("content_index").is_none(),
            "function_call_arguments events must not carry a content_index"
        );
        assert_eq!(events[4].to_json()["name"], "get_weather");
        assert_eq!(events[5].to_json()["item"]["call_id"], "call_x");
    }

    #[tokio::test]
    async fn streaming_error_emits_preamble_then_failed() {
        let transformer = streaming_transformer();

        let events = transformer
            .transform(token_message(GeneratedTokenResult::ChatTemplateError(
                "boom".to_owned(),
            )))
            .await
            .unwrap();

        assert_eq!(
            names(&events),
            vec![
                "response.created",
                "response.in_progress",
                "response.failed"
            ]
        );

        let failed = events[2].to_json();

        assert_eq!(failed["response"]["status"], "failed");
        assert_eq!(failed["response"]["error"]["code"], "server_error");
        assert_eq!(failed["response"]["error"]["message"], "boom");
    }

    #[tokio::test]
    async fn non_streaming_aggregates_content_into_a_message_item() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hel".to_owned(),
            )))
            .await
            .unwrap();
        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "lo".to_owned(),
            )))
            .await
            .unwrap();
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(3, 2, 0),
            )))
            .await
            .unwrap();

        let TransformResult::Chunk(body) = &chunks[0] else {
            panic!("expected a chunk");
        };
        let response: serde_json::Value = serde_json::from_str(body).unwrap();

        assert_eq!(response["object"], "response");
        assert_eq!(response["status"], "completed");
        assert_eq!(response["output"][0]["type"], "message");
        assert_eq!(response["output"][0]["content"][0]["text"], "hello");
    }

    #[tokio::test]
    async fn non_streaming_surfaces_reasoning_and_tool_calls_in_output() {
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ReasoningToken(
                "ponder".to_owned(),
            )))
            .await
            .unwrap();
        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .unwrap();
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(3, 0, 1),
            )))
            .await
            .unwrap();

        let TransformResult::Chunk(body) = &chunks[0] else {
            panic!("expected a chunk");
        };
        let response: serde_json::Value = serde_json::from_str(body).unwrap();

        assert_eq!(response["output"][0]["type"], "reasoning");
        assert_eq!(response["output"][1]["type"], "function_call");
        assert_eq!(response["output"][1]["name"], "get_weather");
        assert_eq!(
            response["usage"]["output_tokens_details"]["reasoning_tokens"],
            1
        );
    }

    #[tokio::test]
    async fn non_streaming_error_returns_an_error_envelope() {
        let transformer = non_streaming_transformer();

        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::SamplerError(
                "sampler blew up".to_owned(),
            )))
            .await
            .unwrap();

        let TransformResult::Error(body) = &chunks[0] else {
            panic!("expected an error");
        };

        assert!(body.contains("sampler blew up"));
        assert!(body.contains("server_error"));
    }

    fn prepared_from(value: serde_json::Value) -> super::ResponsesPreparedRequest {
        let params: OpenAIResponsesRequestParams = serde_json::from_value(value).unwrap();

        params.into_prepared().unwrap()
    }

    #[test]
    fn string_input_becomes_a_single_user_message() {
        let prepared = prepared_from(json!({ "model": "test", "input": "Say hello" }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content.text_content(), "Say hello");
    }

    #[test]
    fn instructions_are_prepended_as_a_system_message() {
        let prepared = prepared_from(json!({
            "model": "test",
            "instructions": "be terse",
            "input": "hi"
        }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].content.text_content(), "be terse");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn function_call_output_item_becomes_a_tool_message() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": [
                { "type": "function_call_output", "call_id": "call_1", "output": "sunny" }
            ]
        }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages[0].role, "tool");
        assert_eq!(messages[0].content.text_content(), "sunny");
    }

    #[test]
    fn developer_role_is_normalized_to_system() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": [
                { "type": "message", "role": "developer", "content": "rules" }
            ]
        }));

        assert_eq!(
            prepared.paddler_params.conversation_history.messages[0].role,
            "system"
        );
    }

    #[test]
    fn flat_function_tool_maps_to_an_internal_tool_with_default_description() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "tools": [
                { "type": "function", "name": "get_weather", "parameters": { "type": "object" } }
            ]
        }));

        assert!(prepared.paddler_params.parse_tool_calls);

        let Tool::Function(function_call) = &prepared.paddler_params.tools[0];

        assert_eq!(function_call.function.name, "get_weather");
        assert_eq!(function_call.function.description, "");
    }

    #[test]
    fn text_format_json_schema_becomes_a_grammar_constraint() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "text": { "format": { "type": "json_schema", "name": "out", "schema": { "type": "object" } } }
        }));

        let Some(GrammarConstraint::JsonSchema { schema }) = &prepared.paddler_params.grammar
        else {
            panic!("expected a json schema grammar constraint");
        };

        assert!(schema.contains("\"type\":\"object\""));
    }

    #[test]
    fn reasoning_effort_none_disables_thinking() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "reasoning": { "effort": "none" }
        }));

        assert!(!prepared.paddler_params.enable_thinking);
    }

    #[test]
    fn unsupported_tool_is_skipped_and_disables_tool_call_parsing() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "tools": [ { "type": "web_search" } ]
        }));

        assert!(prepared.paddler_params.tools.is_empty());
        assert!(!prepared.paddler_params.parse_tool_calls);
    }

    #[tokio::test]
    async fn every_emitted_streaming_event_conforms_to_the_official_schema() {
        let validator = OpenAIValidator::new().unwrap();
        let transformer = streaming_transformer();

        let mut emitted: Vec<ResponsesStreamEvent> = Vec::new();

        for token in [
            GeneratedTokenResult::ReasoningToken("ponder".to_owned()),
            GeneratedTokenResult::ContentToken("hello".to_owned()),
            GeneratedTokenResult::ToolCallParsed(vec![weather_call()]),
            GeneratedTokenResult::Done(summary_with_counts(5, 3, 2)),
        ] {
            emitted.extend(transformer.transform(token_message(token)).await.unwrap());
        }

        assert!(emitted.len() > 10);

        for event in &emitted {
            validator
                .validate_responses_stream_event(&event.to_json())
                .unwrap();
        }
    }

    #[tokio::test]
    async fn the_failed_streaming_event_conforms_to_the_official_schema() {
        let validator = OpenAIValidator::new().unwrap();
        let transformer = streaming_transformer();

        let events = transformer
            .transform(token_message(GeneratedTokenResult::SamplerError(
                "boom".to_owned(),
            )))
            .await
            .unwrap();

        for event in &events {
            validator
                .validate_responses_stream_event(&event.to_json())
                .unwrap();
        }
    }

    #[tokio::test]
    async fn the_non_streaming_response_conforms_to_the_official_schema() {
        let validator = OpenAIValidator::new().unwrap();
        let transformer = non_streaming_transformer();

        transformer
            .transform(token_message(GeneratedTokenResult::ReasoningToken(
                "p".to_owned(),
            )))
            .await
            .unwrap();
        transformer
            .transform(token_message(GeneratedTokenResult::ContentToken(
                "hello".to_owned(),
            )))
            .await
            .unwrap();
        transformer
            .transform(token_message(GeneratedTokenResult::ToolCallParsed(vec![
                weather_call(),
            ])))
            .await
            .unwrap();
        let chunks = transformer
            .transform(token_message(GeneratedTokenResult::Done(
                summary_with_counts(5, 3, 2),
            )))
            .await
            .unwrap();

        let TransformResult::Chunk(body) = &chunks[0] else {
            panic!("expected a chunk");
        };
        let response: serde_json::Value = serde_json::from_str(body).unwrap();

        validator.validate_responses_response(&response).unwrap();
    }

    #[test]
    fn unsupported_and_stateful_fields_are_ignored() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "store": true,
            "previous_response_id": "resp_prev",
            "conversation": "conv_1",
            "temperature": 0.5,
            "tool_choice": "required"
        }));

        assert_eq!(
            prepared.paddler_params.conversation_history.messages.len(),
            1
        );
    }
}
