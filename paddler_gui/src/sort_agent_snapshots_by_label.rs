use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;

pub fn sort_agent_snapshots_by_label(agent_snapshots: &mut [AgentControllerSnapshot]) {
    agent_snapshots.sort_by(|left_agent, right_agent| {
        let left_label = left_agent.name.as_deref().unwrap_or(&left_agent.id);
        let right_label = right_agent.name.as_deref().unwrap_or(&right_agent.id);

        left_label.cmp(right_label)
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

    use super::sort_agent_snapshots_by_label;

    fn agent_snapshot(id: &str, name: Option<&str>) -> AgentControllerSnapshot {
        AgentControllerSnapshot {
            desired_slots_total: 0,
            download_current: 0,
            download_filename: None,
            download_indeterminate: false,
            download_total: 0,
            id: id.to_owned(),
            issues: BTreeSet::new(),
            model_path: None,
            name: name.map(str::to_owned),
            slots_processing: 0,
            slots_total: 0,
            state_application_status: AgentStateApplicationStatus::Fresh,
            uses_chat_template_override: false,
        }
    }

    #[test]
    fn orders_agents_by_name_falling_back_to_their_id() {
        let mut agent_snapshots = vec![
            agent_snapshot("id_z", Some("alpha")),
            agent_snapshot("id_m", None),
            agent_snapshot("id_a", Some("bravo")),
        ];

        sort_agent_snapshots_by_label(&mut agent_snapshots);

        assert_eq!(
            agent_snapshots
                .iter()
                .map(|agent_snapshot| agent_snapshot.id.as_str())
                .collect::<Vec<_>>(),
            vec!["id_z", "id_a", "id_m"]
        );
    }
}
