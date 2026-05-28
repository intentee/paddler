use anyhow::Result;
use log::warn;
use paddler_client::PaddlerClient;
use tokio::process::Child;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_handle_params::ClusterHandleParams;
use crate::terminate_child::terminate_child;

pub struct ClusterHandle {
    pub addresses: BalancerAddresses,
    pub agent_ids: Vec<String>,
    pub agents: AgentsStreamWatcher,
    pub buffered_requests: BufferedRequestsStreamWatcher,
    pub paddler_client: PaddlerClient,
    pub cancel_token: CancellationToken,
    agent_subprocesses: Vec<Child>,
    balancer_subprocess: Child,
}

impl ClusterHandle {
    #[must_use]
    pub fn new(
        ClusterHandleParams {
            addresses,
            agent_ids,
            agent_subprocesses,
            agents,
            balancer_subprocess,
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
            agent_subprocesses,
            balancer_subprocess,
        }
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.cancel_token.cancel();

        for child in self.agent_subprocesses.iter_mut() {
            terminate_child(child)?;
        }

        terminate_child(&mut self.balancer_subprocess)?;

        for agent in self.agent_subprocesses.iter_mut() {
            agent.wait().await?;
        }

        self.balancer_subprocess.wait().await?;

        Ok(())
    }
}

impl Drop for ClusterHandle {
    fn drop(&mut self) {
        self.cancel_token.cancel();

        for child in self.agent_subprocesses.iter_mut() {
            if let Err(error) = terminate_child(child) {
                warn!("ClusterHandle drop: failed to terminate agent subprocess: {error:#}");
            }
        }
        if let Err(error) = terminate_child(&mut self.balancer_subprocess) {
            warn!("ClusterHandle drop: failed to terminate balancer subprocess: {error:#}");
        }
    }
}
