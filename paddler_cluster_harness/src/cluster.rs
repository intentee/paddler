use std::future::Future;

use anyhow::Context as _;
use anyhow::Result;
use paddler::balancer::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler::balancer::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler::request_params::GenerateEmbeddingBatchParams;
use paddler::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_client::PaddlerClient;
use reqwest::Client;
use serde_json::Value;

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
use crate::inference_http_client::InferenceHttpClient;
use crate::inference_message_stream::InferenceMessageStream;
use crate::openai_chat_completions_client::OpenAIChatCompletionsClient;
use crate::running_agent::RunningAgent;
use crate::running_balancer::RunningBalancer;
use crate::wait_until_healthy::wait_until_healthy;

pub struct Cluster {
    pub agent_ids: Vec<String>,
    pub agents: Vec<RunningAgent>,
    pub agents_watcher: AgentsStreamWatcher,
    pub balancer: RunningBalancer,
    pub buffered_requests_watcher: BufferedRequestsStreamWatcher,
    pub paddler_client: PaddlerClient,
    agent_spawner: Box<dyn AgentSpawner>,
    inference_client: InferenceHttpClient,
    openai_client: OpenAIChatCompletionsClient,
}

impl Cluster {
    pub async fn connect(
        balancer: RunningBalancer,
        agent_spawner: Box<dyn AgentSpawner>,
        desired_state: Option<&BalancerDesiredState>,
    ) -> Result<Self> {
        let management_base_url = balancer.addresses.management_base_url()?;
        let inference_base_url = balancer.addresses.inference_base_url()?;
        let openai_base_url = balancer.addresses.compat_openai_base_url()?;

        wait_until_healthy(&management_base_url, "health")
            .await
            .context("balancer did not become healthy")?;

        let paddler_client = PaddlerClient::new(inference_base_url.clone(), management_base_url, 1);

        if let Some(desired_state) = desired_state {
            paddler_client
                .management()
                .put_balancer_desired_state(desired_state)
                .await
                .map_err(anyhow::Error::new)
                .context("failed to PUT balancer desired state")?;
        }

        let agents_watcher = AgentsStreamWatcher::connect(&paddler_client.management()).await?;
        let buffered_requests_watcher =
            BufferedRequestsStreamWatcher::connect(&paddler_client.management()).await?;

        let http_client = Client::new();
        let inference_client = InferenceHttpClient::new(http_client.clone(), inference_base_url);
        let openai_client = OpenAIChatCompletionsClient::new(http_client, &openai_base_url)?;

        Ok(Self {
            agent_ids: Vec::new(),
            agents: Vec::new(),
            agents_watcher,
            balancer,
            buffered_requests_watcher,
            paddler_client,
            agent_spawner,
            inference_client,
            openai_client,
        })
    }

    pub fn continue_from_raw_prompt(
        &self,
        params: &ContinueFromRawPromptParams,
    ) -> impl Future<Output = Result<CollectedGeneratedTokens>> + Send + use<> {
        let inference_client = self.inference_client.clone();
        let params = params.clone();

        async move {
            collect_generated_tokens(
                inference_client
                    .post_continue_from_raw_prompt(&params)
                    .await?,
            )
            .await
        }
    }

    pub fn continue_from_raw_prompt_stream(
        &self,
        params: &ContinueFromRawPromptParams,
    ) -> impl Future<Output = Result<InferenceMessageStream>> + Send + use<> {
        let inference_client = self.inference_client.clone();
        let params = params.clone();

        async move {
            inference_client
                .post_continue_from_raw_prompt(&params)
                .await
        }
    }

    pub fn continue_from_conversation_history(
        &self,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> impl Future<Output = Result<CollectedGeneratedTokens>> + Send + use<> {
        let inference_client = self.inference_client.clone();
        let params = params.clone();

        async move {
            collect_generated_tokens(
                inference_client
                    .post_continue_from_conversation_history(&params)
                    .await?,
            )
            .await
        }
    }

    pub fn continue_from_conversation_history_stream(
        &self,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> impl Future<Output = Result<InferenceMessageStream>> + Send + use<> {
        let inference_client = self.inference_client.clone();
        let params = params.clone();

        async move {
            inference_client
                .post_continue_from_conversation_history(&params)
                .await
        }
    }

    pub fn generate_embedding_batch(
        &self,
        params: &GenerateEmbeddingBatchParams,
    ) -> impl Future<Output = Result<CollectedEmbeddingResults>> + Send + use<> {
        let inference_client = self.inference_client.clone();
        let params = params.clone();

        async move {
            collect_embedding_results(
                inference_client
                    .post_generate_embedding_batch(&params)
                    .await?,
            )
            .await
        }
    }

    pub fn generate_embedding_batch_stream(
        &self,
        params: &GenerateEmbeddingBatchParams,
    ) -> impl Future<Output = Result<InferenceMessageStream>> + Send + use<> {
        let inference_client = self.inference_client.clone();
        let params = params.clone();

        async move {
            inference_client
                .post_generate_embedding_batch(&params)
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
