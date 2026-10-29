use crate::generated_tool_call::GeneratedToolCall;
use crate::raw_tool_call_tokens::RawToolCallTokens;

#[derive(Debug, Eq, PartialEq)]
pub enum GeneratedTokenPartialResult {
    ContentToken(String),
    ReasoningToken(String),
    ToolCallParseFailed(String),
    ToolCallParsed(Vec<GeneratedToolCall>),
    ToolCallToken(String),
    ToolCallValidationFailed(Vec<String>),
    UndeterminableToken(String),
    UnrecognizedToolCallFormat(RawToolCallTokens),
}
