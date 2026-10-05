use anyhow::Context as _;
use anyhow::Result;
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use futures_util::StreamExt as _;
use serde_json::Value;
use url::Url;

use crate::openai_config_from_base_url::openai_config_from_base_url;

#[derive(Clone)]
pub struct OpenAIResponsesClient {
    client: Client<OpenAIConfig>,
}

impl OpenAIResponsesClient {
    pub fn new(openai_base_url: &Url) -> Result<Self> {
        Ok(Self {
            client: Client::with_config(openai_config_from_base_url(openai_base_url)?),
        })
    }

    pub async fn post_streaming(&self, body: &Value) -> Result<Vec<Value>> {
        let mut streaming_body = body.clone();

        if let Some(object) = streaming_body.as_object_mut() {
            object.insert("stream".to_owned(), Value::Bool(true));
        }

        let mut stream = self
            .client
            .responses()
            .create_stream_byot::<Value, Value>(streaming_body)
            .await
            .context("failed to start OpenAI streaming response")?;

        let mut events: Vec<Value> = Vec::new();

        while let Some(event) = stream.next().await {
            events.push(event.context("OpenAI streaming response event failed")?);
        }

        Ok(events)
    }

    pub async fn post_non_streaming(&self, body: &Value) -> Result<Value> {
        self.client
            .responses()
            .create_byot::<&Value, Value>(body)
            .await
            .context("OpenAI non-streaming response failed")
    }
}
