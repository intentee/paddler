use crate::chat_completion_header::ChatCompletionHeader;

pub struct ChatCompletionStreamParams {
    pub header: ChatCompletionHeader,
    pub include_usage: bool,
    pub system_fingerprint: String,
}
