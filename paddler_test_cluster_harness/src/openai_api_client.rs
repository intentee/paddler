use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use futures_util::TryStreamExt as _;
use serde_json::Value;
use url::ParseError;
use url::Url;

#[derive(Clone)]
pub struct OpenAIApiClient {
    client: Client<OpenAIConfig>,
}

impl OpenAIApiClient {
    pub fn new(openai_base_url: &Url) -> Result<Self, ParseError> {
        Ok(Self {
            client: Client::with_config(
                OpenAIConfig::default()
                    .with_api_base(openai_base_url.join("v1")?)
                    .with_api_key("paddler"),
            ),
        })
    }

    pub async fn chat_completion_non_streaming(&self, body: &Value) -> Result<Value, OpenAIError> {
        self.client.chat().create_byot(body).await
    }

    pub async fn chat_completion_streaming(&self, body: &Value) -> Result<Vec<Value>, OpenAIError> {
        self.client
            .chat()
            .create_stream_byot::<&Value, Value>(body)
            .await?
            .try_collect()
            .await
    }

    pub async fn responses_non_streaming(&self, body: &Value) -> Result<Value, OpenAIError> {
        self.client.responses().create_byot(body).await
    }

    pub async fn responses_streaming(&self, body: &Value) -> Result<Vec<Value>, OpenAIError> {
        self.client
            .responses()
            .create_stream_byot::<&Value, Value>(body)
            .await?
            .try_collect()
            .await
    }
}
