use llama_cpp_bindings_types::ParsedToolCall;
use serde::Serialize;
use serde::Serializer;

use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;

const ASSISTANT_ROLE: &str = "assistant";

#[derive(Serialize)]
struct FunctionDelta<'call> {
    name: &'call str,
    arguments: String,
}

#[derive(Serialize)]
struct ToolCallDelta<'call> {
    index: usize,
    id: &'call str,
    #[serde(rename = "type")]
    call_type: &'static str,
    function: FunctionDelta<'call>,
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
                    id: &call.id,
                    call_type: "function",
                    function: FunctionDelta {
                        name: &call.name,
                        arguments: arguments_to_tool_call_string(&call.arguments),
                    },
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
    logprobs: Option<()>,
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
                logprobs: None,
                finish_reason: None,
            },
            Self::Finish(finish_reason) => SerializedChoice {
                index: 0,
                delta: ChoiceDelta::Finish {},
                logprobs: None,
                finish_reason: Some(finish_reason),
            },
            Self::ToolCalls(parsed_calls) => SerializedChoice {
                index: 0,
                delta: ChoiceDelta::ToolCalls {
                    role: ASSISTANT_ROLE,
                    tool_calls: ToolCallsDelta(parsed_calls),
                },
                logprobs: None,
                finish_reason: None,
            },
        }
        .serialize(serializer)
    }
}
