use anyhow::Context as _;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::start_cluster::start_cluster;

struct ClusterWithARegisteredAgent {
    agent_id: String,
    cluster: Cluster,
}

impl ClusterWithARegisteredAgent {
    fn of(cluster: Cluster) -> Result<Self> {
        cluster
            .agent_ids
            .first()
            .cloned()
            .context("the cluster must register its agent")
            .map(|agent_id| Self { agent_id, cluster })
    }

    async fn put(self, next_desired_state: &BalancerDesiredState) -> Result<Self> {
        self.cluster
            .client_management
            .put_balancer_desired_state(CancellationToken::new(), next_desired_state)
            .await
            .context("the balancer must accept the next state")
            .map(|()| self)
    }

    async fn until_the_initial_state_is_applied(mut self) -> Result<Self> {
        self.cluster
            .agents_watcher
            .until_agent(&self.agent_id, |AgentControllerPoolSnapshot { agents }| {
                agents.iter().all(|AgentControllerSnapshot { status, .. }| {
                    status.state_application_status == AgentStateApplicationStatus::Applied
                })
            })
            .await
            .context("the agent must apply the initial state")
            .map(|_applied| self)
    }

    async fn until_the_next_state_is_being_applied(mut self) -> Result<Cluster> {
        self.cluster
            .agents_watcher
            .until_agent(&self.agent_id, |AgentControllerPoolSnapshot { agents }| {
                agents.iter().any(|AgentControllerSnapshot { status, .. }| {
                    status.state_application_status != AgentStateApplicationStatus::Applied
                })
            })
            .await
            .context("the agent must start applying the next state")
            .map(|_applying| self.cluster)
    }
}

pub struct ShutdownDuringStateTransition {
    pub initial_desired_state: BalancerDesiredState,
    pub next_desired_state: BalancerDesiredState,
    pub slot_count: u16,
}

impl ShutdownDuringStateTransition {
    /// # Errors
    ///
    /// Returns an error when the cluster cannot start or register its agent, when the agent does not apply the initial state, when the balancer does not accept the next state, when the agent does not start applying it, and when the cluster does not shut down cleanly while its agent applies the next state.
    pub async fn shut_down_while_the_agent_applies_the_next_state(self) -> Result<()> {
        let Self {
            initial_desired_state,
            next_desired_state,
            slot_count,
        } = self;

        start_cluster(ClusterParams {
            agents: vec![AgentConfig::single(slot_count)],
            desired_state: ClusterDesiredState::Apply(Box::new(initial_desired_state)),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        })
        .map(|started| started.context("the cluster must start"))
        .and_then(|cluster| ready(ClusterWithARegisteredAgent::of(cluster)))
        .and_then(ClusterWithARegisteredAgent::until_the_initial_state_is_applied)
        .and_then(
            |cluster_with_a_registered_agent: ClusterWithARegisteredAgent| {
                cluster_with_a_registered_agent.put(&next_desired_state)
            },
        )
        .and_then(ClusterWithARegisteredAgent::until_the_next_state_is_being_applied)
        .and_then(|cluster: Cluster| {
            cluster.shutdown().map(|shut_down: Result<()>| {
                shut_down.context(
                    "the cluster must shut down cleanly while its agent applies the next state",
                )
            })
        })
        .await
    }
}
