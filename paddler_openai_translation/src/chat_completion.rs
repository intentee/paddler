use llama_cpp_bindings_types::ParsedToolCall;
use serde::Serialize;
use serde::Serializer;

use crate::assistant_role::ASSISTANT_ROLE;
use crate::chat_completion_tool_call::ChatCompletionToolCall;
use crate::chat_completion_usage::ChatCompletionUsage;

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
    usage: ChatCompletionUsage,
    service_tier: &'static str,
}

pub struct ChatCompletion {
    pub content: String,
    pub created: u64,
    pub finish_reason: &'static str,
    pub id: String,
    pub model: String,
    pub tool_calls: Vec<ParsedToolCall>,
    pub usage: ChatCompletionUsage,
}

impl Serialize for ChatCompletion {
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
                content: &self.content,
                refusal: (),
                annotations: [],
            }
        } else {
            SerializedMessage::ToolCalls {
                role: ASSISTANT_ROLE,
                content: (!self.content.is_empty()).then_some(self.content.as_str()),
                refusal: (),
                annotations: [],
                tool_calls: SerializedToolCalls(&self.tool_calls),
            }
        };

        SerializedCompletion {
            id: &self.id,
            object: "chat.completion",
            created: self.created,
            model: &self.model,
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
