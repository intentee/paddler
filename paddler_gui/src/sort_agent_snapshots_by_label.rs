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
    use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_messaging::agent_status::AgentStatus;

    use super::sort_agent_snapshots_by_label;

    fn agent_snapshot(id: &str, name: Option<&str>) -> AgentControllerSnapshot {
        AgentControllerSnapshot {
            id: id.to_owned(),
            name: name.map(str::to_owned),
            slots_processing: 0,
            status: AgentStatus::default(),
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
