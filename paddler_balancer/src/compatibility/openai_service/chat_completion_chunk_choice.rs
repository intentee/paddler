use llama_cpp_bindings_types::ParsedToolCall;
use serde::Serialize;
use serde::Serializer;

use crate::compatibility::openai_service::assistant_role::ASSISTANT_ROLE;
use crate::compatibility::openai_service::chat_completion_tool_call::ChatCompletionToolCall;

#[derive(Serialize)]
struct ToolCallDelta<'call> {
    index: usize,
    #[serde(flatten)]
    tool_call: ChatCompletionToolCall<'call>,
}

struct ToolCallsDelta<'calls>(&'calls [ParsedToolCall]);

impl Serialize for ToolCallsDelta<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        serializer.collect_seq(
            self.0
                .iter()
                .enumerate()
                .map(|(index, call)| ToolCallDelta {
                    index,
                    tool_call: ChatCompletionToolCall(call),
                }),
        )
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum ChoiceDelta<'choice> {
    Content {
        role: &'static str,
        content: &'choice str,
    },
    Finish {},
    ToolCalls {
        role: &'static str,
        tool_calls: ToolCallsDelta<'choice>,
    },
}

#[derive(Serialize)]
struct SerializedChoice<'choice> {
    index: u8,
    delta: ChoiceDelta<'choice>,
    logprobs: (),
    finish_reason: Option<&'static str>,
}

pub enum ChatCompletionChunkChoice<'chunk> {
    Content(&'chunk str),
    Finish(&'static str),
    ToolCalls(&'chunk [ParsedToolCall]),
}

impl Serialize for ChatCompletionChunkChoice<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        match *self {
            Self::Content(content) => SerializedChoice {
                index: 0,
                delta: ChoiceDelta::Content {
                    role: ASSISTANT_ROLE,
                    content,
                },
                logprobs: (),
                finish_reason: None,
            },
            Self::Finish(finish_reason) => SerializedChoice {
                index: 0,
                delta: ChoiceDelta::Finish {},
                logprobs: (),
                finish_reason: Some(finish_reason),
            },
            Self::ToolCalls(parsed_calls) => SerializedChoice {
                index: 0,
                delta: ChoiceDelta::ToolCalls {
                    role: ASSISTANT_ROLE,
                    tool_calls: ToolCallsDelta(parsed_calls),
                },
                logprobs: (),
                finish_reason: None,
            },
        }
        .serialize(serializer)
    }
}
