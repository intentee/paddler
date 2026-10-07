use llama_cpp_bindings_types::TokenUsage;

use crate::chat_completion_chunk_choice::ChatCompletionChunkChoice;

pub enum ChatCompletionChunkPayload {
    Choice(ChatCompletionChunkChoice),
    Usage(TokenUsage),
}
