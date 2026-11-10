use tokio_util::sync::CancellationToken;
use url::Url;

use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::constants::API_PATH_AGENTS;
use paddler_messaging::constants::API_PATH_AGENTS_STREAM;
use paddler_messaging::constants::API_PATH_BALANCER_APPLICABLE_STATE;
use paddler_messaging::constants::API_PATH_BALANCER_DESIRED_STATE;
use paddler_messaging::constants::API_PATH_BUFFERED_REQUESTS;
use paddler_messaging::constants::API_PATH_BUFFERED_REQUESTS_STREAM;
use paddler_messaging::constants::API_PATH_METRICS;
use paddler_messaging::model_metadata::ModelMetadata;

use crate::agents_stream::AgentsStream;
use crate::buffered_requests_stream::BufferedRequestsStream;
use crate::http_client::HttpClient;
use crate::reports_health::ReportsHealth;
use crate::result::Result;

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

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read, or when the response body is not the expected JSON.
    pub async fn get_agents(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<AgentControllerPoolSnapshot> {
        self.http_client
            .get_json(cancellation_token, API_PATH_AGENTS)
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read.
    pub async fn get_agents_stream(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<AgentsStream> {
        self.http_client
            .get_sse_json(cancellation_token, API_PATH_AGENTS_STREAM)
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read, or when the response body is not the expected JSON.
    pub async fn get_balancer_applicable_state(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<AgentDesiredState> {
        self.http_client
            .get_json(cancellation_token, API_PATH_BALANCER_APPLICABLE_STATE)
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read, or when the response body is not the expected JSON.
    pub async fn get_balancer_desired_state(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<BalancerDesiredState> {
        self.http_client
            .get_json(cancellation_token, API_PATH_BALANCER_DESIRED_STATE)
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read, or when the response body is not the expected JSON.
    pub async fn get_buffered_requests(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<BufferedRequestManagerSnapshot> {
        self.http_client
            .get_json(cancellation_token, API_PATH_BUFFERED_REQUESTS)
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read.
    pub async fn get_buffered_requests_stream(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<BufferedRequestsStream> {
        self.http_client
            .get_sse_json(cancellation_token, API_PATH_BUFFERED_REQUESTS_STREAM)
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read, or when the response body is not the expected JSON.
    pub async fn get_chat_template_override(
        &self,
        cancellation_token: CancellationToken,
        agent_id: &str,
    ) -> Result<Option<ChatTemplate>> {
        self.http_client
            .get_json(
                cancellation_token,
                &ApiPath::agent_chat_template_override(agent_id),
            )
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read, or when the response body cannot be read.
    pub async fn get_metrics(&self, cancellation_token: CancellationToken) -> Result<String> {
        self.http_client
            .get_text(cancellation_token, API_PATH_METRICS)
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read, or when the response body is not the expected JSON.
    pub async fn get_model_metadata(
        &self,
        cancellation_token: CancellationToken,
        agent_id: &str,
    ) -> Result<Option<ModelMetadata>> {
        self.http_client
            .get_json(cancellation_token, &ApiPath::agent_model_metadata(agent_id))
            .await
    }

    /// # Errors
    ///
    /// Returns [`Error::RequestCancelled`](crate::error::Error::RequestCancelled) when the cancellation token fires before the server responds, [`Error::Connect`](crate::error::Error::Connect) when the server cannot be reached, [`Error::ServiceUnavailable`](crate::error::Error::ServiceUnavailable) when the server answers with 503 Service Unavailable, [`Error::UnexpectedResponseStatus`](crate::error::Error::UnexpectedResponseStatus) when it answers with another unsuccessful status, and [`Error::Http`](crate::error::Error::Http) when the request fails in any other way or the body of an unsuccessful response cannot be read, including when the request body cannot be serialized.
    pub async fn put_balancer_desired_state(
        &self,
        cancellation_token: CancellationToken,
        state: &BalancerDesiredState,
    ) -> Result<()> {
        self.http_client
            .put_json(cancellation_token, API_PATH_BALANCER_DESIRED_STATE, state)
            .await
            .map(|_accepted_response| ())
    }
}

impl ReportsHealth for ClientManagement {
    fn http_client(&self) -> &HttpClient {
        &self.http_client
    }
}
