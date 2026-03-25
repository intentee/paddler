pub mod get_models;
pub mod openai_http_stream_from_agent;
pub mod post_chat_completions;

pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
