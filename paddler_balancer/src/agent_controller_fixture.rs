use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_controller::AgentController;
use crate::agent_response_senders::AgentResponseSenders;

pub struct AgentControllerFixture {
    pub agent_controller: Arc<AgentController>,
    pub agent_message_rx: mpsc::UnboundedReceiver<AgentJsonRpcMessage>,
}

impl AgentControllerFixture {
    pub fn reporting(agent_id: &str, reported_status: SlotAggregatedStatusSnapshot) -> Self {
        let (agent_message_tx, agent_message_rx) = mpsc::unbounded_channel();

        Self {
            agent_controller: Arc::new(AgentController {
                agent_message_tx,
                agent_response_senders: AgentResponseSenders::default(),
                connection_close: CancellationToken::new(),
                id: agent_id.to_owned(),
                name: None,
                reported_status: RwLock::new(reported_status),
                slots_processing: AtomicValue::<AtomicU64>::new(0),
            }),
            agent_message_rx,
        }
    }

    pub fn whose_socket_is_gone(
        agent_id: &str,
        reported_status: SlotAggregatedStatusSnapshot,
    ) -> Arc<AgentController> {
        let Self {
            agent_controller,
            agent_message_rx,
        } = Self::reporting(agent_id, reported_status);

        drop(agent_message_rx);

        agent_controller
    }
}
