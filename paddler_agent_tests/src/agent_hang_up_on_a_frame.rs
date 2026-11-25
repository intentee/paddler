use anyhow::Result;
use futures_util::TryFutureExt as _;
use tokio_tungstenite::tungstenite::Message;

use crate::agent_connected_to_an_untrusted_balancer::AgentConnectedToAnUntrustedBalancer;

/// # Errors
///
/// Returns an error when the agent cannot connect to the untrusted balancer, when the frame cannot be sent, and when the agent does not connect again after hanging up.
pub async fn agent_hang_up_on_a_frame(frame: Message) -> Result<()> {
    AgentConnectedToAnUntrustedBalancer::connect()
        .and_then(|connected_agent| connected_agent.send(frame))
        .and_then(AgentConnectedToAnUntrustedBalancer::reconnect_after_the_agent_hangs_up)
        .await
}
