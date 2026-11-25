use paddler_messaging::token_usage::TokenUsage;

use crate::chat_completion_chunk_choice::ChatCompletionChunkChoice;

pub enum ChatCompletionChunkPayload {
    Choice(ChatCompletionChunkChoice),
    Usage(TokenUsage),
}
