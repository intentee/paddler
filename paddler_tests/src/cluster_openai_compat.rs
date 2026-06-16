use anyhow::Result;
use async_trait::async_trait;
use paddler_cluster::cluster::Cluster;
use serde_json::Value;

use crate::openai_chat_completions_client::OpenAIChatCompletionsClient;
use crate::openai_responses_client::OpenAIResponsesClient;

#[async_trait(?Send)]
pub trait ClusterOpenAiCompat {
    async fn openai_chat_completion_streaming(&self, body: &Value) -> Result<Vec<Value>>;

    async fn openai_chat_completion_non_streaming(&self, body: &Value) -> Result<Value>;

    async fn openai_responses_streaming(&self, body: &Value) -> Result<Vec<Value>>;

    async fn openai_responses_non_streaming(&self, body: &Value) -> Result<Value>;
}

#[async_trait(?Send)]
impl ClusterOpenAiCompat for Cluster {
    async fn openai_chat_completion_streaming(&self, body: &Value) -> Result<Vec<Value>> {
        let base_url = self.balancer.addresses.compat_openai_base_url()?;

        OpenAIChatCompletionsClient::new(&base_url)?
            .post_streaming(body)
            .await
    }

    async fn openai_chat_completion_non_streaming(&self, body: &Value) -> Result<Value> {
        let base_url = self.balancer.addresses.compat_openai_base_url()?;

        OpenAIChatCompletionsClient::new(&base_url)?
            .post_non_streaming(body)
            .await
    }

    async fn openai_responses_streaming(&self, body: &Value) -> Result<Vec<Value>> {
        let base_url = self.balancer.addresses.compat_openai_base_url()?;

        OpenAIResponsesClient::new(&base_url)?
            .post_streaming(body)
            .await
    }

    async fn openai_responses_non_streaming(&self, body: &Value) -> Result<Value> {
        let base_url = self.balancer.addresses.compat_openai_base_url()?;

        OpenAIResponsesClient::new(&base_url)?
            .post_non_streaming(body)
            .await
    }
}
