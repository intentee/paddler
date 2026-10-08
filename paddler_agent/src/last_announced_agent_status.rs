use paddler_messaging::agent_status::AgentStatus;

pub struct LastAnnouncedAgentStatus {
    agent_status: AgentStatus,
}

impl LastAnnouncedAgentStatus {
    #[must_use]
    pub const fn new(agent_status: AgentStatus) -> Self {
        Self { agent_status }
    }

    pub fn replace_if_changed(&mut self, agent_status: &AgentStatus) -> bool {
        if self.agent_status == *agent_status {
            return false;
        }

        self.agent_status = agent_status.clone();

        true
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_status::AgentStatus;

    use super::LastAnnouncedAgentStatus;

    #[test]
    fn an_unchanged_status_is_not_announced_again() {
        let mut last_announced_agent_status = LastAnnouncedAgentStatus::new(AgentStatus::default());

        assert!(!last_announced_agent_status.replace_if_changed(&AgentStatus::default()));
    }

    #[test]
    fn a_changed_status_is_announced_once() {
        let mut last_announced_agent_status = LastAnnouncedAgentStatus::new(AgentStatus::default());
        let changed_agent_status = AgentStatus {
            uses_chat_template_override: true,
            ..AgentStatus::default()
        };

        assert!(last_announced_agent_status.replace_if_changed(&changed_agent_status));
        assert!(!last_announced_agent_status.replace_if_changed(&changed_agent_status));
    }
}
