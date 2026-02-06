use std::pin::Pin;

use futures_util::Stream;
use futures_util::StreamExt;
use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use reqwest::Client;
use serde_json::from_str;
use url::Url;

use crate::Result;
use crate::format_api_url::format_api_url;
use crate::stream_sse::StreamSse;

pub struct ClientManagement<'client> {
    url: &'client Url,
    http_client: &'client Client,
}

impl<'client> ClientManagement<'client> {
    pub fn new(url: &'client Url, http_client: &'client Client) -> Self {
        Self { url, http_client }
    }

    pub async fn get_agents(&self) -> Result<AgentControllerPoolSnapshot> {
        let response = self
            .http_client
            .get(format_api_url(self.url.as_str(), "/api/v1/agents"))
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    pub async fn get_balancer_desired_state(&self) -> Result<BalancerDesiredState> {
        let response = self
            .http_client
            .get(format_api_url(
                self.url.as_str(),
                "/api/v1/balancer_desired_state",
            ))
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    pub async fn put_balancer_desired_state(&self, state: &BalancerDesiredState) -> Result<()> {
        self.http_client
            .put(format_api_url(
                self.url.as_str(),
                "/api/v1/balancer_desired_state",
            ))
            .json(state)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn get_buffered_requests(&self) -> Result<BufferedRequestManagerSnapshot> {
        let response = self
            .http_client
            .get(format_api_url(
                self.url.as_str(),
                "/api/v1/buffered_requests",
            ))
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    pub async fn agents_stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<AgentControllerPoolSnapshot>> + Send>>> {
        let response = self
            .http_client
            .get(format_api_url(self.url.as_str(), "/api/v1/agents/stream"))
            .send()
            .await?
            .error_for_status()?;

        let stream = StreamSse::from_response(response)
            .map(|result| result.and_then(|data| from_str(&data).map_err(Into::into)));

        Ok(Box::pin(stream))
    }

    pub async fn buffered_requests_stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<BufferedRequestManagerSnapshot>> + Send>>> {
        let response = self
            .http_client
            .get(format_api_url(
                self.url.as_str(),
                "/api/v1/buffered_requests/stream",
            ))
            .send()
            .await?
            .error_for_status()?;

        let stream = StreamSse::from_response(response)
            .map(|result| result.and_then(|data| from_str(&data).map_err(Into::into)));

        Ok(Box::pin(stream))
    }

    pub async fn get_metrics(&self) -> Result<String> {
        let response = self
            .http_client
            .get(format_api_url(self.url.as_str(), "/metrics"))
            .send()
            .await?
            .error_for_status()?;

        Ok(response.text().await?)
    }
}
