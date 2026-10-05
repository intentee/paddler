use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

pub fn assert_slots_total_at_least(
    agent_id: &str,
    expected_slots_total: i32,
) -> impl Fn(&AgentControllerPoolSnapshot) -> bool {
    let agent_id = agent_id.to_owned();

    move |snapshot| {
        snapshot
            .agents
            .iter()
            .any(|agent| agent.id == agent_id && agent.slots_total >= expected_slots_total)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
    use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

    use super::assert_slots_total_at_least;

    fn snapshot_with(id: &str, slots_total: i32) -> AgentControllerPoolSnapshot {
        AgentControllerPoolSnapshot {
            agents: vec![AgentControllerSnapshot {
                desired_slots_total: slots_total,
                download_current: 0,
                download_filename: None,
                download_indeterminate: false,
                download_total: 0,
                id: id.to_owned(),
                issues: BTreeSet::new(),
                model_path: None,
                name: None,
                slots_processing: 0,
                slots_total,
                state_application_status: AgentStateApplicationStatus::Applied,
                uses_chat_template_override: false,
            }],
        }
    }

    #[test]
    fn matches_when_the_named_agent_has_enough_slots() {
        let predicate = assert_slots_total_at_least("agent-a", 4);

        assert!(predicate(&snapshot_with("agent-a", 4)));
        assert!(predicate(&snapshot_with("agent-a", 6)));
    }

    #[test]
    fn rejects_when_slots_are_below_the_threshold() {
        let predicate = assert_slots_total_at_least("agent-a", 4);

        assert!(!predicate(&snapshot_with("agent-a", 3)));
    }

    #[test]
    fn rejects_when_no_agent_matches_the_id() {
        let predicate = assert_slots_total_at_least("agent-b", 1);

        assert!(!predicate(&snapshot_with("agent-a", 8)));
    }
}
