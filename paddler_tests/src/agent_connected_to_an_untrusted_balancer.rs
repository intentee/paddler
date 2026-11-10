use anyhow::Context as _;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use serde_json::to_string;
use tokio_tungstenite::tungstenite::Message;

use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_test_cluster_harness::untrusted_balancer_socket::UntrustedBalancerSocket;
use paddler_test_cluster_harness::untrusted_balancer_socket_server::UntrustedBalancerSocketServer;

use crate::agent_runner_connected_to::agent_runner_connected_to;

pub struct AgentConnectedToAnUntrustedBalancer {
    pub agent_runner: AgentRunner,
    pub connection: UntrustedBalancerSocket,
    pub untrusted_balancer: UntrustedBalancerSocketServer,
}

impl AgentConnectedToAnUntrustedBalancer {
    /// # Errors
    ///
    /// Returns an error when the untrusted balancer cannot bind, when the agent runner cannot start, and when the agent does not connect.
    pub async fn connect() -> Result<Self> {
        UntrustedBalancerSocketServer::bind()
            .map(|bound| bound.context("the untrusted balancer must bind"))
            .and_then(Self::start_agent_runner)
            .await
    }

    /// # Errors
    ///
    /// Returns an error when the frame cannot be sent.
    pub async fn send(mut self, frame: Message) -> Result<Self> {
        self.connection
            .send(frame)
            .await
            .context("the frame must be sent")
            .map(|()| self)
    }

    /// # Errors
    ///
    /// Returns an error when the agent message cannot be serialized or sent.
    pub async fn send_agent_message(self, agent_message: AgentJsonRpcMessage) -> Result<Self> {
        ready(to_string(&agent_message).context("the agent message must serialize"))
            .and_then(|serialized_agent_message| self.send(Message::text(serialized_agent_message)))
            .await
    }

    async fn accept_agent(
        untrusted_balancer: UntrustedBalancerSocketServer,
        agent_runner: AgentRunner,
    ) -> Result<Self> {
        untrusted_balancer
            .accept_agent()
            .await
            .context("the agent must connect")
            .map(|connection| Self {
                agent_runner,
                connection,
                untrusted_balancer,
            })
    }

    async fn start_agent_runner(untrusted_balancer: UntrustedBalancerSocketServer) -> Result<Self> {
        ready(
            agent_runner_connected_to(untrusted_balancer.management_addr, 1)
                .context("the agent runner must start"),
        )
        .and_then(|agent_runner| Self::accept_agent(untrusted_balancer, agent_runner))
        .await
    }

    /// # Errors
    ///
    /// Returns an error when the agent does not answer over the same connection.
    pub async fn next_agent_response(mut self) -> Result<ResponseEnvelope<AgentJsonRpcResponse>> {
        self.connection
            .next_agent_response()
            .await
            .context("the agent must answer over the same connection")
    }

    /// # Errors
    ///
    /// Returns an error when the agent does not connect again after hanging up.
    pub async fn reconnect_after_the_agent_hangs_up(mut self) -> Result<()> {
        self.connection.wait_for_the_agent_to_hang_up().await;
        self.untrusted_balancer
            .accept_agent()
            .await
            .map(drop)
            .context("the agent must reconnect after ending the connection")
    }
}
