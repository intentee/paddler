use anyhow::Context as _;
use anyhow::Result;
use paddler_client::inference_client::InferenceClient;
use paddler_client::inference_client_params::InferenceClientParams;
use paddler_client::management_client::ManagementClient;
use paddler_client::management_client_params::ManagementClientParams;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

use crate::agent_config::AgentConfig;
use crate::agent_spawner::AgentSpawner;
use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_backend::ClusterBackend;
use crate::cluster_params::ClusterParams;
use crate::provisioned_backend::ProvisionedBackend;
use crate::registered_agent::RegisteredAgent;
use crate::running_balancer::RunningBalancer;
use crate::spawned_agent::SpawnedAgent;
use crate::wait_until_healthy::wait_until_healthy;

pub struct Cluster {
    pub agents: Vec<RegisteredAgent>,
    pub agents_watcher: AgentsStreamWatcher,
    pub balancer: RunningBalancer,
    pub buffered_requests_watcher: BufferedRequestsStreamWatcher,
    pub inference_client: InferenceClient,
    pub management_client: ManagementClient,
    agent_spawner: Box<dyn AgentSpawner>,
}

impl Cluster {
    pub async fn start(
        backend: &dyn ClusterBackend,
        ClusterParams {
            agents,
            desired_state,
            wait_for_slots_ready,
        }: ClusterParams,
    ) -> Result<Self> {
        let ProvisionedBackend {
            agent_spawner,
            running_balancer,
        } = backend.provision().await?;

        let mut cluster =
            Self::connect(running_balancer, agent_spawner, desired_state.as_ref()).await?;

        for agent in &agents {
            cluster.spawn_additional_agent(agent).await?;

            if wait_for_slots_ready {
                cluster
                    .wait_for_agent_ready(&agent.name, agent.slot_count)
                    .await?;
            }
        }

        if !wait_for_slots_ready && !agents.is_empty() {
            cluster.wait_for_agent_count(agents.len()).await?;
        }

        Ok(cluster)
    }

    async fn connect(
        balancer: RunningBalancer,
        agent_spawner: Box<dyn AgentSpawner>,
        desired_state: Option<&BalancerDesiredState>,
    ) -> Result<Self> {
        let management_base_url = balancer.addresses.management_base_url()?;
        let inference_base_url = balancer.addresses.inference_base_url()?;

        wait_until_healthy(&management_base_url, "health")
            .await
            .context("balancer did not become healthy")?;

        let inference_client = InferenceClient::new(InferenceClientParams {
            socket_pool_size: 1,
            url: inference_base_url,
        });
        let management_client = ManagementClient::new(ManagementClientParams {
            url: management_base_url,
        });

        if let Some(desired_state) = desired_state {
            management_client
                .set_desired_state(desired_state)
                .await
                .map_err(anyhow::Error::new)
                .context("failed to PUT balancer desired state")?;
        }

        let agents_watcher = AgentsStreamWatcher::connect(&management_client).await?;
        let buffered_requests_watcher =
            BufferedRequestsStreamWatcher::connect(&management_client).await?;

        Ok(Self {
            agents: Vec::new(),
            agents_watcher,
            balancer,
            buffered_requests_watcher,
            agent_spawner,
            inference_client,
            management_client,
        })
    }

    pub async fn wait_for_agent_count(
        &mut self,
        expected_count: usize,
    ) -> Result<AgentControllerPoolSnapshot> {
        self.agents_watcher
            .until(|snapshot| snapshot.agents.len() == expected_count)
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

    pub async fn wait_for_slots_processing(
        &mut self,
        agent_id: &str,
        expected_slots_processing: i32,
    ) -> Result<AgentControllerPoolSnapshot> {
        let agent_id = agent_id.to_owned();

        self.agents_watcher
            .until(move |snapshot| {
                snapshot.agents.iter().any(|agent| {
                    agent.id == agent_id && agent.slots_processing == expected_slots_processing
                })
            })
            .await
    }

    pub async fn wait_for_buffered_request_count(
        &mut self,
        expected_count: i32,
    ) -> Result<BufferedRequestManagerSnapshot> {
        self.buffered_requests_watcher
            .until(|snapshot| snapshot.buffered_requests_current == expected_count)
            .await
    }

    pub async fn spawn_additional_agent(&mut self, config: &AgentConfig) -> Result<()> {
        let process = self.agent_spawner.spawn(config).await?;
        let spawned = SpawnedAgent::new(config.clone(), process);

        let mut registration_watcher =
            AgentsStreamWatcher::connect(&self.management_client).await?;
        let agent_name = config.name.clone();
        let registration_snapshot = registration_watcher
            .until(move |snapshot| {
                snapshot.agents.iter().any(|snapshot_agent| {
                    snapshot_agent.name.as_deref() == Some(agent_name.as_str())
                })
            })
            .await
            .with_context(|| format!("agent {:?} did not register", config.name))?;

        let id = registration_snapshot
            .agents
            .iter()
            .find(|snapshot_agent| snapshot_agent.name.as_deref() == Some(config.name.as_str()))
            .map(|snapshot_agent| snapshot_agent.id.clone())
            .context("registered agent missing from its registration snapshot")?;

        self.agents.push(spawned.register(id));

        Ok(())
    }

    pub async fn shutdown(self) -> Result<()> {
        for agent in self.agents {
            agent.shutdown().await?;
        }

        self.balancer.shutdown().await
    }
}
