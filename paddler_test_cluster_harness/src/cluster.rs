use std::future::Future;
use std::num::NonZeroUsize;

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_client::client_health::ClientHealth;
use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_client::client_management::ClientManagement;
use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_client::reports_health::ReportsHealth as _;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::agent_config::AgentConfig;
use crate::agent_spawner::AgentSpawner;
use crate::agents_status::assert_agent_count::assert_agent_count;
use crate::agents_status::assert_slots_processing::assert_slots_processing;
use crate::agents_status::assert_slots_total_at_least::assert_slots_total_at_least;
use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::buffered_requests_status::assert_count::assert_count;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::collect_embedding_results::collect_embedding_results;
use crate::collect_generated_tokens::collect_generated_tokens;
use crate::collected_embedding_results::CollectedEmbeddingResults;
use crate::collected_generated_tokens::CollectedGeneratedTokens;
use crate::openai_chat_completions_client::OpenAIChatCompletionsClient;
use crate::openai_responses_client::OpenAIResponsesClient;
use crate::running_agent::RunningAgent;
use crate::running_balancer::RunningBalancer;

const INFERENCE_SOCKET_POOL_SIZE: NonZeroUsize = NonZeroUsize::MIN;

pub struct Cluster {
    pub agent_ids: Vec<String>,
    pub agents: Vec<RunningAgent>,
    pub agents_watcher: AgentsStreamWatcher,
    pub balancer: RunningBalancer,
    pub buffered_requests_watcher: BufferedRequestsStreamWatcher,
    pub client_compat_openai_health: ClientHealth,
    pub client_inference: ClientInference,
    pub client_management: ClientManagement,
    agent_spawner: Box<dyn AgentSpawner>,
    openai_client: OpenAIChatCompletionsClient,
    openai_responses_client: OpenAIResponsesClient,
}

impl Cluster {
    pub async fn connect(
        cancellation_token: CancellationToken,
        balancer: RunningBalancer,
        agent_spawner: Box<dyn AgentSpawner>,
        desired_state: Option<&BalancerDesiredState>,
    ) -> Result<Self> {
        let management_base_url = balancer.addresses.management_base_url()?;
        let inference_base_url = balancer.addresses.inference_base_url()?;
        let openai_base_url = balancer.addresses.compat_openai_base_url()?;

        let client_management = ClientManagement::new(management_base_url);
        let client_inference = ClientInference::new(ClientInferenceParams {
            inference_socket_pool_size: INFERENCE_SOCKET_POOL_SIZE,
            url: inference_base_url,
        });
        let client_compat_openai_health = ClientHealth::new(openai_base_url.clone());

        client_management
            .wait_until_healthy(cancellation_token.clone())
            .await
            .context("balancer did not become healthy")?;

        if let Some(desired_state) = desired_state {
            client_management
                .put_balancer_desired_state(cancellation_token.clone(), desired_state)
                .await
                .context("failed to PUT balancer desired state")?;
        }

        let agents_watcher =
            AgentsStreamWatcher::connect(cancellation_token.clone(), &client_management).await?;
        let buffered_requests_watcher =
            BufferedRequestsStreamWatcher::connect(cancellation_token, &client_management).await?;

        let openai_client = OpenAIChatCompletionsClient::new(&openai_base_url)?;
        let openai_responses_client = OpenAIResponsesClient::new(&openai_base_url)?;

        Ok(Self {
            agent_ids: Vec::new(),
            agents: Vec::new(),
            agents_watcher,
            balancer,
            buffered_requests_watcher,
            client_compat_openai_health,
            client_inference,
            client_management,
            agent_spawner,
            openai_client,
            openai_responses_client,
        })
    }

    pub fn continue_from_raw_prompt(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromRawPromptParams,
    ) -> impl Future<Output = Result<CollectedGeneratedTokens>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        async move {
            collect_generated_tokens(
                client_inference
                    .post_continue_from_raw_prompt(cancellation_token, &params)
                    .await?,
            )
            .await
        }
    }

    pub fn continue_from_raw_prompt_stream(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromRawPromptParams,
    ) -> impl Future<Output = Result<InferenceMessageStream>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        async move {
            Ok(client_inference
                .post_continue_from_raw_prompt(cancellation_token, &params)
                .await?)
        }
    }

    pub fn continue_from_conversation_history(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> impl Future<Output = Result<CollectedGeneratedTokens>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        async move {
            collect_generated_tokens(
                client_inference
                    .post_continue_from_conversation_history(cancellation_token, &params)
                    .await?,
            )
            .await
        }
    }

    pub fn continue_from_conversation_history_stream(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> impl Future<Output = Result<InferenceMessageStream>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        async move {
            Ok(client_inference
                .post_continue_from_conversation_history(cancellation_token, &params)
                .await?)
        }
    }

    pub fn generate_embedding_batch(
        &self,
        cancellation_token: CancellationToken,
        params: &GenerateEmbeddingBatchParams,
    ) -> impl Future<Output = Result<CollectedEmbeddingResults>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        async move {
            collect_embedding_results(
                client_inference
                    .post_generate_embedding_batch(cancellation_token, &params)
                    .await?,
            )
            .await
        }
    }

    pub fn openai_chat_completion_streaming(
        &self,
        body: &Value,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send + use<> {
        let openai_client = self.openai_client.clone();
        let body = body.clone();

        async move { openai_client.post_streaming(&body).await }
    }

    pub fn openai_chat_completion_non_streaming(
        &self,
        body: &Value,
    ) -> impl Future<Output = Result<Value>> + Send + use<> {
        let openai_client = self.openai_client.clone();
        let body = body.clone();

        async move { openai_client.post_non_streaming(&body).await }
    }

    pub fn openai_responses_streaming(
        &self,
        body: &Value,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send + use<> {
        let openai_responses_client = self.openai_responses_client.clone();
        let body = body.clone();

        async move { openai_responses_client.post_streaming(&body).await }
    }

    pub fn openai_responses_non_streaming(
        &self,
        body: &Value,
    ) -> impl Future<Output = Result<Value>> + Send + use<> {
        let openai_responses_client = self.openai_responses_client.clone();
        let body = body.clone();

        async move { openai_responses_client.post_non_streaming(&body).await }
    }

    pub async fn wait_for_agent_count(
        &mut self,
        expected_count: usize,
    ) -> Result<AgentControllerPoolSnapshot> {
        self.agents_watcher
            .until(assert_agent_count(expected_count))
            .await
    }

    pub async fn wait_for_agent_ready(
        &mut self,
        agent_name: &str,
        expected_slot_count: i32,
    ) -> Result<AgentControllerPoolSnapshot> {
        self.agents_watcher
            .wait_for_agent_ready(agent_name, expected_slot_count)
            .await
    }

    pub async fn wait_for_agents_ready(&mut self, expected_slot_counts: &[i32]) -> Result<()> {
        self.agents_watcher
            .wait_for_slots_ready(expected_slot_counts)
            .await
    }

    pub async fn wait_for_slots_processing(
        &mut self,
        agent_id: &str,
        expected_slots_processing: i32,
    ) -> Result<AgentControllerPoolSnapshot> {
        self.agents_watcher
            .until(assert_slots_processing(agent_id, expected_slots_processing))
            .await
    }

    pub async fn wait_for_slots_total_at_least(
        &mut self,
        agent_id: &str,
        expected_slots_total: i32,
    ) -> Result<AgentControllerPoolSnapshot> {
        self.agents_watcher
            .until(assert_slots_total_at_least(agent_id, expected_slots_total))
            .await
    }

    pub async fn wait_for_buffered_request_count(
        &mut self,
        expected_count: i32,
    ) -> Result<BufferedRequestManagerSnapshot> {
        self.buffered_requests_watcher
            .until(assert_count(expected_count))
            .await
    }

    pub fn spawn_additional_agent(&mut self, config: &AgentConfig) -> Result<()> {
        let process = self.agent_spawner.spawn(config)?;

        self.agents.push(RunningAgent::new(config.clone(), process));

        Ok(())
    }

    pub async fn shutdown(self) -> Result<()> {
        for agent in self.agents {
            agent.shutdown().await?;
        }

        self.balancer.shutdown().await
    }
}
