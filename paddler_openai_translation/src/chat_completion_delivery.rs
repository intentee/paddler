use crate::chat_completion_header::ChatCompletionHeader;
use crate::chat_completion_stream::ChatCompletionStream;

pub enum ChatCompletionDelivery {
    Buffered(ChatCompletionHeader),
    Streamed(ChatCompletionStream),
}
