use llama_cpp_bindings_types::ParsedToolCall;
use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeMap as _;
use serde::ser::SerializeSeq as _;

use crate::compatibility::openai_service::arguments_to_tool_call_string::arguments_to_tool_call_string;

struct FunctionDelta<'call>(&'call ParsedToolCall);

impl Serialize for FunctionDelta<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let mut function = serializer.serialize_map(Some(2))?;

        function.serialize_entry("name", &self.0.name)?;
        function.serialize_entry(
            "arguments",
            &arguments_to_tool_call_string(&self.0.arguments),
        )?;
        function.end()
    }
}

struct ToolCallDelta<'call> {
    call: &'call ParsedToolCall,
    index: usize,
}

impl Serialize for ToolCallDelta<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let mut tool_call = serializer.serialize_map(Some(4))?;

        tool_call.serialize_entry("index", &self.index)?;
        tool_call.serialize_entry("id", &self.call.id)?;
        tool_call.serialize_entry("type", "function")?;
        tool_call.serialize_entry("function", &FunctionDelta(self.call))?;
        tool_call.end()
    }
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
        let mut tool_calls = serializer.serialize_seq(Some(self.0.len()))?;

        for (index, call) in self.0.iter().enumerate() {
            tool_calls.serialize_element(&ToolCallDelta { call, index })?;
        }

        tool_calls.end()
    }
}

struct ChoiceDelta<'choice>(&'choice ChatCompletionChunkChoice<'choice>);

impl Serialize for ChoiceDelta<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let mut delta = serializer.serialize_map(None)?;

        match self.0 {
            ChatCompletionChunkChoice::Content(text) => {
                delta.serialize_entry("role", "assistant")?;
                delta.serialize_entry("content", text)?;
            }
            ChatCompletionChunkChoice::Finish(_) => {}
            ChatCompletionChunkChoice::ToolCalls(parsed_calls) => {
                delta.serialize_entry("role", "assistant")?;
                delta.serialize_entry("tool_calls", &ToolCallsDelta(parsed_calls))?;
            }
        }

        delta.end()
    }
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
        let finish_reason = match self {
            Self::Finish(finish_reason) => Some(*finish_reason),
            Self::Content(_) | Self::ToolCalls(_) => None,
        };
        let mut choice = serializer.serialize_map(Some(4))?;

        choice.serialize_entry("index", &0)?;
        choice.serialize_entry("delta", &ChoiceDelta(self))?;
        choice.serialize_entry("logprobs", &None::<()>)?;
        choice.serialize_entry("finish_reason", &finish_reason)?;
        choice.end()
    }
}
