use std::future::Future;

use reqwest::Client;
use reqwest::Error;
use reqwest::Response;
use reqwest::Url;
use serde_json::json;

pub fn minimal_openai_chat_completion(
    chat_completions_url: Url,
) -> impl Future<Output = Result<Response, Error>> + Send {
    Client::new()
        .post(chat_completions_url)
        .json(&json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
}
