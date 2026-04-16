use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;

use crate::running_cluster_data::RunningClusterData;

#[derive(Debug, Clone)]
pub enum Message {
    AgentSnapshotsUpdated(Vec<AgentControllerSnapshot>),
    Stop,
    CopyToClipboard(String),
}

pub enum Action {
    None,
    Stop,
    CopyToClipboard(String),
}

impl RunningClusterData {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::AgentSnapshotsUpdated(snapshots) => {
                self.agent_snapshots = snapshots;

                Action::None
            }
            Message::Stop => {
                self.stopping = true;

                Action::Stop
            }
            Message::CopyToClipboard(content) => Action::CopyToClipboard(content),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

    use super::Action;
    use super::Message;
    use super::RunningClusterData;

    fn make_data() -> RunningClusterData {
        RunningClusterData {
            agent_snapshots: vec![],
            cluster_address: "127.0.0.1:8060".to_owned(),
            stopping: false,
        }
    }

    fn make_snapshot(id: &str) -> AgentControllerSnapshot {
        AgentControllerSnapshot {
            desired_slots_total: 0,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            id: id.to_owned(),
            issues: BTreeSet::new(),
            model_path: None,
            name: None,
            slots_processing: 0,
            slots_total: 0,
            state_application_status: AgentStateApplicationStatus::Fresh,
            uses_chat_template_override: false,
        }
    }

    #[test]
    fn agent_snapshots_updated_replaces_existing_snapshots() {
        let mut data = make_data();
        data.agent_snapshots = vec![make_snapshot("stale")];

        let action = data.update(Message::AgentSnapshotsUpdated(vec![
            make_snapshot("fresh-1"),
            make_snapshot("fresh-2"),
        ]));

        assert!(matches!(action, Action::None));
        assert_eq!(data.agent_snapshots.len(), 2);
        assert_eq!(data.agent_snapshots[0].id, "fresh-1");
    }

    #[test]
    fn stop_marks_stopping_and_returns_stop_action() {
        let mut data = make_data();

        let action = data.update(Message::Stop);

        assert!(matches!(action, Action::Stop));
        assert!(data.stopping);
    }

    #[test]
    fn copy_to_clipboard_returns_action_with_content() {
        let mut data = make_data();

        let action = data.update(Message::CopyToClipboard("paste-me".to_owned()));

        assert!(matches!(action, Action::CopyToClipboard(content) if content == "paste-me"));
    }
}
