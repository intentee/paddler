use std::future::Future;
use std::num::NonZeroUsize;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use log::LevelFilter;
use log::set_max_level;
use serde_json::Value;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_model_source::constants::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_hub::HuggingFaceHub;
use paddler_client::client_health::ClientHealth;
use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_client::client_management::ClientManagement;
use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_client::reports_health::ReportsHealth as _;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::model_download_status::ModelDownloadStatus;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::agent_config::AgentConfig;
use crate::agent_count_is::agent_count_is;
use crate::agent_readiness::AgentReadiness;
use crate::agent_serves_decisions::agent_serves_decisions;
use crate::agent_slots_processing_is::agent_slots_processing_is;
use crate::agent_spawner::AgentSpawner;
use crate::buffered_request_count_is::buffered_request_count_is;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_harness_error::ClusterHarnessError;
use crate::cluster_params::ClusterParams;
use crate::cluster_snapshots_watchers::ClusterSnapshotsWatchers;
use crate::collect_decision_results::collect_decision_results;
use crate::collect_embedding_results::collect_embedding_results;
use crate::collect_generated_tokens::collect_generated_tokens;
use crate::collected_decision_results::CollectedDecisionResults;
use crate::collected_embedding_results::CollectedEmbeddingResults;
use crate::collected_generated_tokens::CollectedGeneratedTokens;
use crate::concurrent_connection_budget::ConcurrentConnectionBudget;
use crate::connection_keep_alive::ConnectionKeepAlive;
use crate::connection_runtime::ConnectionRuntime;
use crate::in_process_agent_spawner::InProcessAgentSpawner;
use crate::in_process_balancer::InProcessBalancer;
use crate::openai_api_client::OpenAIApiClient;
use crate::running_agent::RunningAgent;
use crate::running_balancer::RunningBalancer;
use crate::snapshots_watcher::SnapshotsWatcher;
use crate::typesafe_api_client::TypeSafeApiClient;
use crate::typesafe_api_response::TypeSafeApiResponse;

const INFERENCE_SOCKET_POOL_SIZE: NonZeroUsize = NonZeroUsize::MIN;

async fn open_cluster_connections(
    client_management: ClientManagement,
    cancellation_token: CancellationToken,
    desired_state: ClusterDesiredState,
    connection_keep_alive: ConnectionKeepAlive,
) -> Result<ClusterSnapshotsWatchers> {
    client_management
        .wait_until_healthy(cancellation_token.clone())
        .await
        .context("balancer did not become healthy")?;

    if let ClusterDesiredState::Apply(desired_state) = &desired_state {
        client_management
            .put_balancer_desired_state(cancellation_token.clone(), desired_state)
            .await
            .context("failed to PUT balancer desired state")?;
    }

    ClusterSnapshotsWatchers::open(
        &client_management,
        cancellation_token,
        connection_keep_alive,
    )
    .await
}

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
    connection_runtime: ConnectionRuntime,
}

impl Cluster {
    /// # Errors
    ///
    /// Fails when the balancer does not become healthy, when it refuses the desired state, and when its agents or buffered requests stream cannot be opened, and returns [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when such a stream ends or fails before its first snapshot.
    pub async fn connect(
        cancellation_token: CancellationToken,
        balancer: RunningBalancer,
        agent_spawner: Box<dyn AgentSpawner>,
        desired_state: &ClusterDesiredState,
    ) -> Result<Self> {
        let client_management = ClientManagement::new(balancer.management_base_url.clone());
        let client_inference = ClientInference::new(ClientInferenceParams {
            inference_socket_pool_size: INFERENCE_SOCKET_POOL_SIZE,
            url: balancer.inference_base_url.clone(),
        });

        let connection_runtime = ConnectionRuntime::start()?;
        let ClusterSnapshotsWatchers {
            agents_watcher,
            buffered_requests_watcher,
        } = connection_runtime
            .run(open_cluster_connections(
                client_management.clone(),
                cancellation_token,
                desired_state.clone(),
                connection_runtime.keep_alive(),
            ))
            .await??;

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
            connection_runtime,
        })
    }

    /// # Errors
    ///
    /// Returns an error when the in-process balancer cannot start or be addressed, when the temporary model directory, the model downloader, or the Hugging Face client cannot be created, when the cluster cannot connect to the balancer, and when its agents do not register.
    pub async fn start_in_process(cluster_params: ClusterParams) -> Result<Self> {
        set_max_level(LevelFilter::Trace);

        let running_balancer =
            BalancerRunner::start(cluster_params.in_process_balancer_runner_params()?)
                .await
                .context("failed to start in-process BalancerRunner")
                .and_then(|balancer_runner| {
                    RunningBalancer::new(
                        balancer_runner.addresses,
                        Box::new(InProcessBalancer::new(balancer_runner)),
                    )
                    .context("failed to address the in-process balancer")
                })?;
        let management_address = running_balancer.addresses.management;
        let downloaded_models = TempDir::new()?;
        let desired_model_resolver = DesiredModelResolver {
            cache_dir: CacheDir {
                explicit: Some(downloaded_models.path().to_path_buf()),
                home: None,
                xdg: None,
            },
            download_manager: DownloadManager::new()?,
            huggingface_client: HuggingFaceHub {
                cache_location: HuggingFaceCacheLocation::HubDefault,
                endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
            }
            .client()?,
        };

        let mut cluster = Self::connect(
            CancellationToken::new(),
            running_balancer,
            Box::new(InProcessAgentSpawner {
                desired_model_resolver,
                downloaded_models,
                management_address,
            }),
            &cluster_params.desired_state,
        )
        .await?;

        cluster
            .register_agents(&cluster_params.agents, cluster_params.wait_for_slots_ready)
            .await?;

        Ok(cluster)
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::CompatOpenAIServiceNotServed`] when the balancer does not serve its `OpenAI` compatibility service, and [`ClusterHarnessError::CompatOpenAIBaseUrlInvalid`] when its base URL cannot be built.
    pub fn compat_openai_health_client(&self) -> Result<ClientHealth, ClusterHarnessError> {
        self.balancer
            .compat_openai_base_url()
            .map(ClientHealth::new)
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::CompatTypeSafeServiceNotServed`] when the balancer does not serve its `TypeSafe` compatibility service, and [`ClusterHarnessError::CompatTypeSafeBaseUrlInvalid`] when its base URL cannot be built.
    pub fn compat_typesafe_health_client(&self) -> Result<ClientHealth, ClusterHarnessError> {
        self.balancer
            .compat_typesafe_base_url()
            .map(ClientHealth::new)
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

    /// # Errors
    ///
    /// Fails when an agent cannot be spawned, and returns [`ClusterHarnessError::AgentExitedBeforeReady`] when an agent exits before it is ready, [`ClusterHarnessError::AgentReportedIssues`] when it reports issues while its slots are awaited, and [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when the agents stream ends or fails before it is ready.
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

    /// # Errors
    ///
    /// Fails when an agent or the balancer cannot be terminated, or ends with a failure.
    pub async fn shutdown(self) -> Result<()> {
        let Self {
            agents,
            agents_watcher,
            balancer,
            buffered_requests_watcher,
            connection_runtime,
            ..
        } = self;

        for agent in agents {
            agent.shutdown().await?;
        }

        balancer.shutdown().await?;
        drop(agents_watcher);
        drop(buffered_requests_watcher);
        connection_runtime.shut_down().await;

        Ok(())
    }

    /// # Errors
    ///
    /// Fails when the agent cannot be spawned.
    pub fn spawn_additional_agent(&mut self, config: &AgentConfig) -> Result<()> {
        let process = self.agent_spawner.spawn(config)?;

        self.agents.push(RunningAgent::new(config.clone(), process));

        Ok(())
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

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when the agents stream ends or fails before the pool holds the expected number of agents.
    pub async fn wait_for_agent_count(
        &mut self,
        expected_count: usize,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.agents_watcher
            .until(agent_count_is(expected_count))
            .await
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when the buffered requests stream ends or fails before the balancer buffers the expected number of requests.
    pub async fn wait_for_buffered_request_count(
        &mut self,
        expected_count: u64,
    ) -> Result<BufferedRequestManagerSnapshot, ClusterHarnessError> {
        self.buffered_requests_watcher
            .until(buffered_request_count_is(expected_count))
            .await
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::AgentLeftThePool`] when the agent leaves the pool, and [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when the agents stream ends or fails before the agent uses its chat template override.
    pub async fn wait_for_chat_template_override_in_use(
        &mut self,
        agent_id: &str,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.agents_watcher
            .until_agent(agent_id, |AgentControllerPoolSnapshot { agents }| {
                agents
                    .iter()
                    .any(|AgentControllerSnapshot { id, status, .. }| {
                        id == agent_id && status.uses_chat_template_override
                    })
            })
            .await
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::ClusterHasNoRegisteredAgent`] when no agent is registered, [`ClusterHarnessError::AgentLeftThePool`] when the first agent leaves the pool, and [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when the agents stream ends or fails before the first agent reports a matching issue.
    pub async fn wait_for_first_agent_issue<TIssueMatcher>(
        &mut self,
        issue_matcher: TIssueMatcher,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError>
    where
        TIssueMatcher: Fn(&AgentIssue) -> bool + Send + Sync,
    {
        self.wait_for_first_agent(|AgentControllerSnapshot { status, .. }| {
            status.issues.iter().any(&issue_matcher)
        })
        .await
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::ClusterHasNoRegisteredAgent`] when no agent is registered, and [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when the agents stream ends or fails before the first agent downloads its model.
    pub async fn wait_for_first_agent_to_download(
        &mut self,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.wait_for_first_agent(|AgentControllerSnapshot { status, .. }| {
            status.download_status != ModelDownloadStatus::NotDownloading
        })
        .await
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::ClusterHasNoRegisteredAgent`] when no agent is registered, [`ClusterHarnessError::AgentLeftThePool`] when the first agent leaves the pool, and [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when the agents stream ends or fails before the first agent serves the decisions.
    pub async fn wait_for_first_agent_to_serve(
        &mut self,
        decisions: AgentDecisionsServing,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError> {
        self.wait_for_first_agent(|agent| agent_serves_decisions(agent, decisions))
            .await
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::SnapshotsStreamClosed`] or [`ClusterHarnessError::SnapshotsStreamFailed`] when the agents stream ends or fails before the agent processes the expected number of slots.
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

    async fn wait_for_first_agent<TAgentMatcher>(
        &mut self,
        agent_matcher: TAgentMatcher,
    ) -> Result<AgentControllerPoolSnapshot, ClusterHarnessError>
    where
        TAgentMatcher: Fn(&AgentControllerSnapshot) -> bool + Send + Sync,
    {
        let agent_id = self
            .agent_ids
            .first()
            .ok_or(ClusterHarnessError::ClusterHasNoRegisteredAgent)?
            .clone();

        self.agents_watcher
            .until_agent(&agent_id, |AgentControllerPoolSnapshot { agents }| {
                agents
                    .iter()
                    .any(|agent| agent.id == agent_id && agent_matcher(agent))
            })
            .await
    }
}
