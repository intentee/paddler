use std::future::Future;

use reqwest::Error;
use reqwest::Response;
use reqwest::Url;
use serde_json::json;

use crate::post_openai_request::post_openai_request;

pub fn minimal_openai_chat_completion(
    chat_completions_url: Url,
) -> impl Future<Output = Result<Response, Error>> + Send {
    post_openai_request(
        chat_completions_url,
        &json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}]
        }),
    )
}
