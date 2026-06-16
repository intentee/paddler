use anyhow::Context as _;
use anyhow::Result;
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use serde_json::Value;
use url::Url;

use crate::collect_openai_stream::collect_openai_stream;
use crate::openai_config_from_base_url::openai_config_from_base_url;
use crate::streaming_request_body::streaming_request_body;

#[derive(Clone)]
pub struct OpenAIChatCompletionsClient {
    client: Client<OpenAIConfig>,
}

impl OpenAIChatCompletionsClient {
    pub fn new(openai_base_url: &Url) -> Result<Self> {
        Ok(Self {
            client: Client::with_config(openai_config_from_base_url(openai_base_url)?),
        })
    }

    pub async fn post_streaming(&self, body: &Value) -> Result<Vec<Value>> {
        let stream = self
            .client
            .chat()
            .create_stream_byot::<Value, Value>(streaming_request_body(body))
            .await
            .context("failed to start OpenAI streaming chat completion")?;

        collect_openai_stream(stream).await
    }

    pub async fn post_non_streaming(&self, body: &Value) -> Result<Value> {
        self.client
            .chat()
            .create_byot::<&Value, Value>(body)
            .await
            .context("OpenAI non-streaming chat completion failed")
    }
}
