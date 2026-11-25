use std::future::Future;

use reqwest::Client;
use reqwest::Error;
use reqwest::Response;
use reqwest::Url;
use serde_json::Value;

pub fn post_openai_chat_completion(
    chat_completions_url: Url,
    body: &Value,
) -> impl Future<Output = Result<Response, Error>> + Send + use<> {
    Client::new().post(chat_completions_url).json(body).send()
}
