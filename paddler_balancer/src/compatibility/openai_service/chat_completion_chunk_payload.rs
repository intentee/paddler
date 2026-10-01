use llama_cpp_bindings_types::TokenUsage;

use crate::compatibility::openai_service::chat_completion_chunk_choice::ChatCompletionChunkChoice;

pub enum ChatCompletionChunkPayload<'chunk> {
    Choice(ChatCompletionChunkChoice<'chunk>),
    Usage(&'chunk TokenUsage),
}
