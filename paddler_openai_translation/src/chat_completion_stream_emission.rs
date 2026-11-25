use crate::chat_completion_chunk::ChatCompletionChunk;
use crate::generation_failure::GenerationFailure;

pub enum ChatCompletionStreamEmission {
    Chunks(Vec<ChatCompletionChunk>),
    Failure(GenerationFailure),
}
