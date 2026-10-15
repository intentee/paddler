use std::future::Future;
use std::num::NonZeroUsize;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use paddler_client::client_health::ClientHealth;
use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_client::client_management::ClientManagement;
use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_client::reports_health::ReportsHealth as _;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::agent_config::AgentConfig;
use crate::agent_count_is::agent_count_is;
use crate::agent_readiness::AgentReadiness;
use crate::agent_slots_processing_is::agent_slots_processing_is;
use crate::agent_spawner::AgentSpawner;
use crate::buffered_request_count_is::buffered_request_count_is;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_harness_error::ClusterHarnessError;
use crate::collect_decision_results::collect_decision_results;
use crate::collect_embedding_results::collect_embedding_results;
use crate::collect_generated_tokens::collect_generated_tokens;
use crate::collected_decision_results::CollectedDecisionResults;
use crate::collected_embedding_results::CollectedEmbeddingResults;
use crate::collected_generated_tokens::CollectedGeneratedTokens;
use crate::concurrent_connection_budget::ConcurrentConnectionBudget;
use crate::openai_api_client::OpenAIApiClient;
use crate::running_agent::RunningAgent;
use crate::running_balancer::RunningBalancer;
use crate::snapshots_watcher::SnapshotsWatcher;
use crate::typesafe_api_client::TypeSafeApiClient;
use crate::typesafe_api_response::TypeSafeApiResponse;

const INFERENCE_SOCKET_POOL_SIZE: NonZeroUsize = NonZeroUsize::MIN;

pub struct Cluster {
    pub agent_ids: Vec<String>,
    pub agents: Vec<RunningAgent>,
    pub agents_watcher: SnapshotsWatcher<AgentControllerPoolSnapshot>,
    pub balancer: RunningBalancer,
    pub buffered_requests_watcher: SnapshotsWatcher<BufferedRequestManagerSnapshot>,
    pub client_inference: ClientInference,
    pub client_management: ClientManagement,
    agent_spawner: Box<dyn AgentSpawner>,
    connection_budget: ConcurrentConnectionBudget,
}

impl Cluster {
    pub async fn connect(
        cancellation_token: CancellationToken,
        balancer: RunningBalancer,
        agent_spawner: Box<dyn AgentSpawner>,
        desired_state: &ClusterDesiredState,
    ) -> Result<Self> {
        let management_base_url = balancer.management_base_url()?;
        let inference_base_url = balancer.inference_base_url()?;

        let client_management = ClientManagement::new(management_base_url);
        let client_inference = ClientInference::new(ClientInferenceParams {
            inference_socket_pool_size: INFERENCE_SOCKET_POOL_SIZE,
            url: inference_base_url,
        });

        client_management
            .wait_until_healthy(cancellation_token.clone())
            .await
            .context("balancer did not become healthy")?;

        if let ClusterDesiredState::Apply(desired_state) = desired_state {
            client_management
                .put_balancer_desired_state(cancellation_token.clone(), desired_state)
                .await
                .context("failed to PUT balancer desired state")?;
        }

        let agents_watcher = SnapshotsWatcher::of_agents(
            client_management
                .get_agents_stream(cancellation_token.clone())
                .await
                .context("failed to open the agents stream")?,
        )
        .await?;
        let buffered_requests_watcher = SnapshotsWatcher::of_buffered_requests(
            client_management
                .get_buffered_requests_stream(cancellation_token)
                .await
                .context("failed to open the buffered requests stream")?,
        )
        .await?;

        Ok(Self {
            agent_ids: Vec::new(),
            agents: Vec::new(),
            agents_watcher,
            balancer,
            buffered_requests_watcher,
            client_inference,
            client_management,
            agent_spawner,
            connection_budget: ConcurrentConnectionBudget::default(),
        })
    }

    pub fn continue_from_raw_prompt(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromRawPromptParams,
    ) -> impl Future<Output = Result<CollectedGeneratedTokens>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        self.connection_budget.hold_during(async move {
            collect_generated_tokens(
                client_inference
                    .post_continue_from_raw_prompt(cancellation_token, &params)
                    .await?,
            )
            .await
        })
    }

    pub fn continue_from_raw_prompt_stream(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromRawPromptParams,
    ) -> impl Future<Output = Result<InferenceMessageStream, ClusterHarnessError>> + Send + use<>
    {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        self.connection_budget.hold_while_streaming(async move {
            client_inference
                .post_continue_from_raw_prompt(cancellation_token, &params)
                .await
        })
    }

    pub fn continue_from_conversation_history(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> impl Future<Output = Result<CollectedGeneratedTokens>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        self.connection_budget.hold_during(async move {
            collect_generated_tokens(
                client_inference
                    .post_continue_from_conversation_history(cancellation_token, &params)
                    .await?,
            )
            .await
        })
    }

    pub fn continue_from_conversation_history_stream(
        &self,
        cancellation_token: CancellationToken,
        params: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> impl Future<Output = Result<InferenceMessageStream, ClusterHarnessError>> + Send + use<>
    {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        self.connection_budget.hold_while_streaming(async move {
            client_inference
                .post_continue_from_conversation_history(cancellation_token, &params)
                .await
        })
    }

    pub fn decide(
        &self,
        cancellation_token: CancellationToken,
        params: &RawDecideParams,
    ) -> impl Future<Output = Result<CollectedDecisionResults>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        self.connection_budget.hold_during(async move {
            collect_decision_results(
                client_inference
                    .post_decide(cancellation_token, &params)
                    .await?,
            )
            .await
        })
    }

    pub fn generate_embedding_batch(
        &self,
        cancellation_token: CancellationToken,
        params: &GenerateEmbeddingBatchParams,
    ) -> impl Future<Output = Result<CollectedEmbeddingResults>> + Send + use<> {
        let client_inference = self.client_inference.clone();
        let params = params.clone();

        self.connection_budget.hold_during(async move {
            collect_embedding_results(
                client_inference
                    .post_generate_embedding_batch(cancellation_token, &params)
                    .await?,
            )
            .await
        })
    }

    pub fn compat_openai_health_client(&self) -> Result<ClientHealth, ClusterHarnessError> {
        self.balancer
            .compat_openai_base_url()
            .map(ClientHealth::new)
    }

    pub fn compat_typesafe_health_client(&self) -> Result<ClientHealth, ClusterHarnessError> {
        self.balancer
            .compat_typesafe_base_url()
            .map(ClientHealth::new)
    }

    pub fn openai_chat_completion_streaming(
        &self,
        body: &Value,
    ) -> impl Future<Output = Result<Vec<Value>, ClusterHarnessError>> + Send + use<> {
        let body = body.clone();

        self.connection_budget
            .hold_during(
                ready(self.openai_api_client()).and_then(|openai_api_client| async move {
                    openai_api_client
                        .chat_completion_streaming(&body)
                        .await
                        .map_err(ClusterHarnessError::OpenAIRequestFailed)
                }),
            )
    }

    pub fn openai_chat_completion_non_streaming(
        &self,
        body: &Value,
    ) -> impl Future<Output = Result<Value, ClusterHarnessError>> + Send + use<> {
        let body = body.clone();

        self.connection_budget
            .hold_during(
                ready(self.openai_api_client()).and_then(|openai_api_client| async move {
                    openai_api_client
                        .chat_completion_non_streaming(&body)
                        .await
                        .map_err(ClusterHarnessError::OpenAIRequestFailed)
                }),
            )
    }

    pub fn openai_responses_streaming(
        &self,
        body: &Value,
    ) -> impl Future<Output = Result<Vec<Value>, ClusterHarnessError>> + Send + use<> {
        let body = body.clone();

        self.connection_budget
            .hold_during(
                ready(self.openai_api_client()).and_then(|openai_api_client| async move {
                    openai_api_client
                        .responses_streaming(&body)
                        .await
                        .map_err(ClusterHarnessError::OpenAIRequestFailed)
                }),
            )
    }

    pub fn openai_responses_non_streaming(
        &self,
        body: &Value,
    ) -> impl Future<Output = Result<Value, ClusterHarnessError>> + Send + use<> {
        let body = body.clone();

        self.connection_budget
            .hold_during(
                ready(self.openai_api_client()).and_then(|openai_api_client| async move {
                    openai_api_client
                        .responses_non_streaming(&body)
                        .await
                        .map_err(ClusterHarnessError::OpenAIRequestFailed)
                }),
            )
    }

    pub fn typesafe_models(
        &self,
    ) -> impl Future<Output = Result<TypeSafeApiResponse, ClusterHarnessError>> + Send + use<> {
        self.connection_budget
            .hold_during(ready(self.typesafe_api_client()).and_then(
                |typesafe_api_client| async move {
                    typesafe_api_client
                        .models()
                        .await
                        .map_err(ClusterHarnessError::TypeSafeRequestFailed)
                },
            ))
    }

    pub fn typesafe_system_one(
        &self,
        request_id: &str,
        body: &Value,
    ) -> impl Future<Output = Result<TypeSafeApiResponse, ClusterHarnessError>> + Send + use<> {
        let request_id = request_id.to_owned();
        let body = body.clone();

        self.connection_budget
            .hold_during(ready(self.typesafe_api_client()).and_then(
                |typesafe_api_client| async move {
                    typesafe_api_client
                        .system_one(&request_id, &body)
                        .await
                        .map_err(ClusterHarnessError::TypeSafeRequestFailed)
                },
            ))
    }

    pub async fn wait_for_agent_count(
        &mut self,
        expected_count: usize,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.agents_watcher
            .until(agent_count_is(expected_count))
            .await
    }

    pub async fn wait_for_slots_processing(
        &mut self,
        agent_id: &str,
        expected_slots_processing: u64,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.agents_watcher
            .until(agent_slots_processing_is(
                agent_id,
                expected_slots_processing,
            ))
            .await
    }

    pub async fn wait_for_chat_template_override_in_use(
        &mut self,
        agent_id: &str,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.agents_watcher
            .until_agent(agent_id, |snapshot| {
                snapshot.agents.iter().any(|registered_agent| {
                    registered_agent.id == agent_id
                        && registered_agent.status.uses_chat_template_override
                })
            })
            .await
    }

    pub async fn wait_for_buffered_request_count(
        &mut self,
        expected_count: u64,
    ) -> Result<BufferedRequestManagerSnapshot, ClusterHarnessError> {
        self.buffered_requests_watcher
            .until(buffered_request_count_is(expected_count))
            .await
    }

    pub async fn wait_for_first_agent_issue<TIssueMatcher>(
        &mut self,
        issue_matcher: TIssueMatcher,
    ) -> Result<AgentControllerPoolSnapshot>
    where
        TIssueMatcher: Fn(&AgentIssue) -> bool,
    {
        self.wait_for_first_agent(|agent| agent.status.issues.iter().any(&issue_matcher))
            .await
    }

    pub async fn wait_for_first_agent_to_serve(
        &mut self,
        inference_mode: InferenceMode,
    ) -> Result<AgentControllerPoolSnapshot> {
        self.wait_for_first_agent(|agent| {
            matches!(
                agent.status.runtime,
                AgentRuntimeStatus::Serving {
                    inference_mode: served_inference_mode,
                    ..
                } if served_inference_mode == inference_mode
            )
        })
        .await
    }

    async fn wait_for_first_agent<TAgentMatcher>(
        &mut self,
        agent_matcher: TAgentMatcher,
    ) -> Result<AgentControllerPoolSnapshot>
    where
        TAgentMatcher: Fn(&AgentControllerSnapshot) -> bool,
    {
        let agent_id = self
            .agent_ids
            .first()
            .context("the cluster must have a registered agent")?
            .clone();

        Ok(self
            .agents_watcher
            .until_agent(&agent_id, |snapshot| {
                snapshot
                    .agents
                    .iter()
                    .any(|agent| agent.id == agent_id && agent_matcher(agent))
            })
            .await?)
    }

    pub async fn register_agents(
        &mut self,
        agents: &[AgentConfig],
        wait_for_slots_ready: bool,
    ) -> Result<()> {
        for agent_config in agents {
            let mut running_agent = RunningAgent::new(
                agent_config.clone(),
                self.agent_spawner.spawn(agent_config)?,
            );
            let ready_agent = running_agent
                .wait_until_ready(
                    &mut self.agents_watcher,
                    AgentReadiness::of_registration(agent_config.slot_count, wait_for_slots_ready),
                )
                .await?;

            self.agent_ids.push(ready_agent.id);
            self.agents.push(running_agent);
        }

        Ok(())
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

    fn openai_api_client(&self) -> Result<OpenAIApiClient, ClusterHarnessError> {
        self.balancer
            .compat_openai_base_url()
            .map(OpenAIApiClient::new)
    }

    fn typesafe_api_client(&self) -> Result<TypeSafeApiClient, ClusterHarnessError> {
        self.balancer
            .compat_typesafe_base_url()
            .map(TypeSafeApiClient::new)
    }
}
