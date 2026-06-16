use futures_util::StreamExt as _;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::model_metadata::ModelMetadata;
use reqwest::Client;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde_json::from_str;
use url::Url;

use crate::agents_stream::AgentsStream;
use crate::buffered_requests_stream::BufferedRequestsStream;
use crate::error::Result;
use crate::format_api_url::format_api_url;
use crate::management_client_params::ManagementClientParams;
use crate::stream::sse::Sse;

pub struct ManagementClient {
    http_client: Client,
    url: Url,
}

impl ManagementClient {
    #[must_use]
    pub fn new(ManagementClientParams { url }: ManagementClientParams) -> Self {
        Self {
            http_client: Client::new(),
            url,
        }
    }

    async fn get(&self, path: &str) -> Result<Response> {
        Ok(self
            .http_client
            .get(format_api_url(&self.url, path))
            .send()
            .await?
            .error_for_status()?)
    }

    async fn get_text(&self, path: &str) -> Result<String> {
        Ok(self.get(path).await?.text().await?)
    }

    async fn get_json<TResponse: DeserializeOwned>(&self, path: &str) -> Result<TResponse> {
        Ok(self.get(path).await?.json().await?)
    }

    pub async fn health(&self) -> Result<String> {
        self.get_text("/health").await
    }

    pub async fn agents(&self) -> Result<AgentControllerPoolSnapshot> {
        self.get_json("/api/v1/agents").await
    }

    pub async fn desired_state(&self) -> Result<BalancerDesiredState> {
        self.get_json("/api/v1/balancer_desired_state").await
    }

    pub async fn applicable_state(&self) -> Result<Option<AgentDesiredState>> {
        self.get_json("/api/v1/balancer_applicable_state").await
    }

    pub async fn set_desired_state(&self, state: &BalancerDesiredState) -> Result<()> {
        self.http_client
            .put(format_api_url(&self.url, "/api/v1/balancer_desired_state"))
            .json(state)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn buffered_requests(&self) -> Result<BufferedRequestManagerSnapshot> {
        self.get_json("/api/v1/buffered_requests").await
    }

    pub async fn agents_stream(&self) -> Result<AgentsStream> {
        let response = self.get("/api/v1/agents/stream").await?;

        let stream = Sse::from_response(response)
            .map(|result| result.and_then(|data| from_str(&data).map_err(Into::into)));

        Ok(Box::pin(stream))
    }

    pub async fn buffered_requests_stream(&self) -> Result<BufferedRequestsStream> {
        let response = self.get("/api/v1/buffered_requests/stream").await?;

        let stream = Sse::from_response(response)
            .map(|result| result.and_then(|data| from_str(&data).map_err(Into::into)));

        Ok(Box::pin(stream))
    }

    pub async fn chat_template_override(&self, agent_id: &str) -> Result<Option<ChatTemplate>> {
        self.get_json(&format!("/api/v1/agent/{agent_id}/chat_template_override"))
            .await
    }

    pub async fn model_metadata(&self, agent_id: &str) -> Result<Option<ModelMetadata>> {
        self.get_json(&format!("/api/v1/agent/{agent_id}/model_metadata"))
            .await
    }

    pub async fn metrics(&self) -> Result<String> {
        self.get_text("/metrics").await
    }
}
