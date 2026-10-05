use llama_cpp_bindings_types::ParsedToolCall;
use serde::Serialize;
use serde::Serializer;

use crate::compatibility::openai_service::assistant_role::ASSISTANT_ROLE;
use crate::compatibility::openai_service::chat_completion_tool_call::ChatCompletionToolCall;
use crate::compatibility::openai_service::openai_usage::OpenAIUsage;

struct SerializedToolCalls<'calls>(&'calls [ParsedToolCall]);

impl Serialize for SerializedToolCalls<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        serializer.collect_seq(self.0.iter().map(ChatCompletionToolCall))
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum SerializedMessage<'completion> {
    Content {
        role: &'static str,
        content: &'completion str,
        refusal: (),
        annotations: [u8; 0],
    },
    ToolCalls {
        role: &'static str,
        content: Option<&'completion str>,
        refusal: (),
        annotations: [u8; 0],
        tool_calls: SerializedToolCalls<'completion>,
    },
}

#[derive(Serialize)]
struct SerializedChoice<'completion> {
    index: u8,
    message: SerializedMessage<'completion>,
    logprobs: (),
    finish_reason: &'static str,
}

#[derive(Serialize)]
struct SerializedCompletion<'completion> {
    id: &'completion str,
    object: &'static str,
    created: u64,
    model: &'completion str,
    choices: [SerializedChoice<'completion>; 1],
    usage: OpenAIUsage,
    service_tier: &'static str,
}

pub struct ChatCompletion<'completion> {
    pub content: &'completion str,
    pub created: u64,
    pub finish_reason: &'static str,
    pub id: &'completion str,
    pub model: &'completion str,
    pub tool_calls: &'completion [ParsedToolCall],
    pub usage: OpenAIUsage,
}

impl Serialize for ChatCompletion<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let message = if self.tool_calls.is_empty() {
            SerializedMessage::Content {
                role: ASSISTANT_ROLE,
                content: self.content,
                refusal: (),
                annotations: [],
            }
        } else {
            SerializedMessage::ToolCalls {
                role: ASSISTANT_ROLE,
                content: (!self.content.is_empty()).then_some(self.content),
                refusal: (),
                annotations: [],
                tool_calls: SerializedToolCalls(self.tool_calls),
            }
        };

        SerializedCompletion {
            id: self.id,
            object: "chat.completion",
            created: self.created,
            model: self.model,
            choices: [SerializedChoice {
                index: 0,
                message,
                logprobs: (),
                finish_reason: self.finish_reason,
            }],
            usage: self.usage,
            service_tier: "default",
        }
        .serialize(serializer)
    }
}
