use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::management_connection_step::ManagementConnectionStep;

pub struct LastAnnouncedAgentStatus {
    agent_status: AgentStatus,
}

impl LastAnnouncedAgentStatus {
    #[must_use]
    pub const fn new(agent_status: AgentStatus) -> Self {
        Self { agent_status }
    }

    pub fn announce_if_changed(
        &mut self,
        slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot,
    ) -> ManagementConnectionStep {
        if self.agent_status == slot_aggregated_status_snapshot.status {
            return ManagementConnectionStep::Continue;
        }

        self.agent_status = slot_aggregated_status_snapshot.status.clone();

        ManagementConnectionStep::Write(ManagementJsonRpcMessage::Notification(
            ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                slot_aggregated_status_snapshot,
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_status::AgentStatus;
    use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
    use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
    use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::LastAnnouncedAgentStatus;
    use crate::management_connection_step::ManagementConnectionStep;

    #[test]
    fn a_changed_status_is_announced_once() {
        let mut last_announced_agent_status = LastAnnouncedAgentStatus::new(AgentStatus::default());
        let changed_snapshot = SlotAggregatedStatusSnapshot {
            status: AgentStatus {
                uses_chat_template_override: true,
                ..AgentStatus::default()
            },
            version: 1,
        };

        assert!(matches!(
            last_announced_agent_status.announce_if_changed(changed_snapshot.clone()),
            ManagementConnectionStep::Write(ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                    slot_aggregated_status_snapshot,
                }),
            )) if slot_aggregated_status_snapshot == changed_snapshot
        ));
        assert!(matches!(
            last_announced_agent_status.announce_if_changed(changed_snapshot),
            ManagementConnectionStep::Continue
        ));
    }
}
