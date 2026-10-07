use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_connection::BalancerConnection;

use crate::agent_running_action::AgentRunningAction;
use crate::agent_running_message::AgentRunningMessage;

pub struct AgentRunningData {
    pub balancer_address: String,
    pub balancer_connection: BalancerConnection,
    pub cancellation_token: CancellationToken,
    pub name: Option<String>,
    pub slots_processing: u64,
    pub status: AgentStatus,
}

impl AgentRunningData {
    pub fn update(&mut self, message: AgentRunningMessage) -> AgentRunningAction {
        match message {
            AgentRunningMessage::AgentStatusUpdated {
                slots_processing,
                status,
            } => {
                self.slots_processing = slots_processing;
                self.status = status;

                AgentRunningAction::None
            }
            AgentRunningMessage::BalancerConnectionChanged(balancer_connection) => {
                self.balancer_connection = balancer_connection;

                AgentRunningAction::None
            }
            AgentRunningMessage::Disconnect => {
                self.cancellation_token.cancel();

                AgentRunningAction::Disconnect
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::agent_status::AgentStatus;
    use paddler_messaging::balancer_connection::BalancerConnection;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::AgentRunningData;
    use crate::agent_running_action::AgentRunningAction;
    use crate::agent_running_message::AgentRunningMessage;

    const AGENT_NAME: &str = "gpu-box";

    fn agent_waiting_for_its_first_status() -> AgentRunningData {
        AgentRunningData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            balancer_connection: BalancerConnection::Connecting,
            cancellation_token: CancellationToken::new(),
            name: Some(AGENT_NAME.to_owned()),
            slots_processing: 0,
            status: AgentStatus::default(),
        }
    }

    #[test]
    fn a_reported_status_updates_the_agent_under_its_own_name() {
        let mut agent = agent_waiting_for_its_first_status();
        let reported_status = AgentStatus {
            desired_slots_total: 4,
            model_path: Some("/models/qwen3.gguf".to_owned()),
            runtime: AgentRuntimeStatus::Serving {
                inference_mode: InferenceMode::TextGeneration,
                slots_total: 4,
            },
            state_application_status: AgentStateApplicationStatus::Applied,
            ..AgentStatus::default()
        };

        let action = agent.update(AgentRunningMessage::AgentStatusUpdated {
            slots_processing: 1,
            status: reported_status.clone(),
        });

        assert_eq!(action, AgentRunningAction::None);
        assert_eq!(agent.name.as_deref(), Some(AGENT_NAME));
        assert_eq!(agent.slots_processing, 1);
        assert_eq!(agent.status, reported_status);
    }

    #[test]
    fn a_reported_status_alone_does_not_mark_the_agent_connected() {
        let mut agent = agent_waiting_for_its_first_status();

        agent.update(AgentRunningMessage::AgentStatusUpdated {
            slots_processing: 0,
            status: AgentStatus::default(),
        });

        assert_eq!(agent.balancer_connection, BalancerConnection::Connecting);
    }

    #[test]
    fn an_opened_balancer_connection_marks_the_agent_connected() {
        let mut agent = agent_waiting_for_its_first_status();

        agent.update(AgentRunningMessage::BalancerConnectionChanged(
            BalancerConnection::Connected,
        ));

        assert_eq!(agent.balancer_connection, BalancerConnection::Connected);
    }

    #[test]
    fn disconnecting_cancels_the_running_agent() {
        let mut agent = agent_waiting_for_its_first_status();

        let action = agent.update(AgentRunningMessage::Disconnect);

        assert_eq!(action, AgentRunningAction::Disconnect);
        assert!(agent.cancellation_token.is_cancelled());
    }
}
