use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use futures_util::TryStreamExt as _;
use serde_json::Value;
use url::Url;

const OPENAI_API_BASE_PATH: &str = "/v1";

#[derive(Clone)]
pub struct OpenAIApiClient {
    client: Client<OpenAIConfig>,
}

impl OpenAIApiClient {
    #[must_use]
    pub fn new(mut openai_base_url: Url) -> Self {
        openai_base_url.set_path(OPENAI_API_BASE_PATH);

        Self {
            client: Client::with_config(
                OpenAIConfig::default()
                    .with_api_base(openai_base_url)
                    .with_api_key("paddler"),
            ),
        }
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
