use futures_util::StreamExt;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::model_metadata::ModelMetadata;
use serde_json::from_str;
use url::Url;

use crate::agents_stream::AgentsStream;
use crate::buffered_requests_stream::BufferedRequestsStream;
use crate::error::Result;
use crate::http_client::HttpClient;
use crate::reports_health::ReportsHealth;
use crate::stream::sse::Sse;

#[derive(Clone)]
pub struct ClientManagement {
    http_client: HttpClient,
}

impl ClientManagement {
    #[must_use]
    pub fn new(url: Url) -> Self {
        Self {
            http_client: HttpClient::new(url),
        }
    }

    pub async fn get_agents(&self) -> Result<AgentControllerPoolSnapshot> {
        self.http_client.get_json("/api/v1/agents").await
    }

    pub async fn get_balancer_desired_state(&self) -> Result<BalancerDesiredState> {
        self.http_client
            .get_json("/api/v1/balancer_desired_state")
            .await
    }

    pub async fn get_balancer_applicable_state(&self) -> Result<Option<AgentDesiredState>> {
        self.http_client
            .get_json("/api/v1/balancer_applicable_state")
            .await
    }

    pub async fn put_balancer_desired_state(&self, state: &BalancerDesiredState) -> Result<()> {
        self.http_client
            .put_json("/api/v1/balancer_desired_state", state)
            .await?;

        Ok(())
    }

    pub async fn get_buffered_requests(&self) -> Result<BufferedRequestManagerSnapshot> {
        self.http_client.get_json("/api/v1/buffered_requests").await
    }

    pub async fn get_agents_stream(&self) -> Result<AgentsStream> {
        let response = self.http_client.get("/api/v1/agents/stream").await?;

        let stream = Sse::from_response(response)
            .map(|result| result.and_then(|data| from_str(&data).map_err(Into::into)));

        Ok(Box::pin(stream))
    }

    pub async fn get_buffered_requests_stream(&self) -> Result<BufferedRequestsStream> {
        let response = self
            .http_client
            .get("/api/v1/buffered_requests/stream")
            .await?;

        let stream = Sse::from_response(response)
            .map(|result| result.and_then(|data| from_str(&data).map_err(Into::into)));

        Ok(Box::pin(stream))
    }

    pub async fn get_chat_template_override(&self, agent_id: &str) -> Result<Option<ChatTemplate>> {
        self.http_client
            .get_json(&format!("/api/v1/agent/{agent_id}/chat_template_override"))
            .await
    }

    pub async fn get_model_metadata(&self, agent_id: &str) -> Result<Option<ModelMetadata>> {
        self.http_client
            .get_json(&format!("/api/v1/agent/{agent_id}/model_metadata"))
            .await
    }

    pub async fn get_metrics(&self) -> Result<String> {
        self.http_client.get_text("/metrics").await
    }
}

impl ReportsHealth for ClientManagement {
    fn http_client(&self) -> &HttpClient {
        &self.http_client
    }
}
