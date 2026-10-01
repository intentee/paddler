use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::notification_params::set_state_params::SetStateParams;

#[must_use]
pub fn agent_desired_state_notification(desired_state: AgentDesiredState) -> AgentJsonRpcMessage {
    AgentJsonRpcMessage::Notification(AgentJsonRpcNotification::SetState(Box::new(
        SetStateParams { desired_state },
    )))
}
