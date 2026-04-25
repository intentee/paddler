use anyhow::Result;
use paddler_client::PaddlerClient;
use tokio_util::sync::CancellationToken;

use crate::agents_stream_watcher::AgentsStreamWatcher;
use crate::balancer_addresses::BalancerAddresses;
use crate::buffered_requests_stream_watcher::BufferedRequestsStreamWatcher;
use crate::cluster_completion::ClusterCompletion;
use crate::cluster_handle_params::ClusterHandleParams;
use crate::send_sigterm_if_running::send_sigterm_if_running;

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

    pub async fn shutdown(self) -> Result<()> {
        let Self {
            cancel_token,
            completion,
            ..
        } = self;

        cancel_token.cancel();

        match completion {
            ClusterCompletion::InProcess {
                mut agents,
                mut balancer,
            } => {
                for agent_runner in &mut agents {
                    agent_runner.wait_for_completion().await?;
                }

                balancer.wait_for_completion().await?;
            }
            ClusterCompletion::Subprocess {
                mut agents,
                mut balancer,
            } => {
                for child in &agents {
                    send_sigterm_if_running(child)?;
                }

                send_sigterm_if_running(&balancer)?;

                for agent in &mut agents {
                    agent.wait().await?;
                }

                balancer.wait().await?;
            }
        }

        Ok(())
    }
}
