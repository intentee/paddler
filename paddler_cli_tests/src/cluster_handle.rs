use anyhow::Result;
use log::warn;
use paddler_client::PaddlerClient;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_completion::ClusterCompletion;
use crate::cluster_handle_params::ClusterHandleParams;
use crate::terminate_child::terminate_child;

pub struct ClusterHandle {
    pub addresses: BalancerAddresses,
    pub agent_ids: Vec<String>,
    pub agents: AgentsStreamWatcher,
    pub buffered_requests: BufferedRequestsStreamWatcher,
    pub paddler_client: PaddlerClient,
    pub cancel_token: CancellationToken,
    completion: ClusterCompletion,
}

impl ClusterHandle {
    #[must_use]
    pub fn new(
        ClusterHandleParams {
            addresses,
            agent_ids,
            agents,
            buffered_requests,
            cancel_token,
            completion,
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
            completion,
        }
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.cancel_token.cancel();

        match &mut self.completion {
            ClusterCompletion::InProcess { agents, balancer } => {
                for agent_runner in agents.iter_mut() {
                    agent_runner.wait_for_completion().await?;
                }

                balancer.wait_for_completion().await?;
            }
            ClusterCompletion::Subprocess { agents, balancer } => {
                for child in agents.iter_mut() {
                    terminate_child(child)?;
                }

                terminate_child(balancer)?;

                for agent in agents.iter_mut() {
                    agent.wait().await?;
                }

                balancer.wait().await?;
            }
        }

        Ok(())
    }
}

impl Drop for ClusterHandle {
    fn drop(&mut self) {
        self.cancel_token.cancel();

        if let ClusterCompletion::Subprocess { agents, balancer } = &mut self.completion {
            for child in agents.iter_mut() {
                if let Err(error) = terminate_child(child) {
                    warn!("ClusterHandle drop: failed to terminate agent subprocess: {error:#}");
                }
            }
            if let Err(error) = terminate_child(balancer) {
                warn!("ClusterHandle drop: failed to terminate balancer subprocess: {error:#}");
            }
        }
    }
}
