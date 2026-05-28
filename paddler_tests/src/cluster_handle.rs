use anyhow::Result;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_client::PaddlerClient;
use tokio_util::sync::CancellationToken;

use crate::agent_config::AgentConfig;
use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_handle_params::ClusterHandleParams;

pub struct ClusterHandle {
    pub addresses: BalancerAddresses,
    pub agent_ids: Vec<String>,
    pub agents: AgentsStreamWatcher,
    pub buffered_requests: BufferedRequestsStreamWatcher,
    pub paddler_client: PaddlerClient,
    pub cancel_token: CancellationToken,
    agent_runners: Vec<AgentRunner>,
    balancer_runner: BalancerRunner,
}

impl ClusterHandle {
    #[must_use]
    pub fn new(
        ClusterHandleParams {
            addresses,
            agent_ids,
            agent_runners,
            agents,
            balancer_runner,
            buffered_requests,
            cancel_token,
            paddler_client,
        }: ClusterHandleParams,
    ) -> Self {
        Self {
            addresses,
            agent_ids,
            agents,
            buffered_requests,
            paddler_client,
            cancel_token,
            agent_runners,
            balancer_runner,
        }
    }

    pub fn spawn_additional_agent(&mut self, config: &AgentConfig) {
        let agent_runner = AgentRunner::start(AgentRunnerParams {
            agent_name: Some(config.name.clone()),
            management_address: self.addresses.management.to_string(),
            cancellation_token: self.cancel_token.clone(),
            slots: config.slot_count,
        });

        self.agent_runners.push(agent_runner);
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.cancel_token.cancel();

        for agent_runner in &mut self.agent_runners {
            agent_runner.wait_for_completion().await?;
        }

        self.balancer_runner.wait_for_completion().await?;

        Ok(())
    }
}

impl Drop for ClusterHandle {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}
