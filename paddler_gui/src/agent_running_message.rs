use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_connection::BalancerConnection;

#[derive(Debug, Clone)]
pub enum AgentRunningMessage {
    AgentStatusUpdated {
        slots_processing: u64,
        status: AgentStatus,
    },
    BalancerConnectionChanged(BalancerConnection),
    Disconnect,
}
