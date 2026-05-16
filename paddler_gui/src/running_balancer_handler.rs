use crate::running_balancer_data::RunningBalancerData;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;

#[derive(Debug, Clone)]
pub enum Message {
    SnapshotUpdated(Box<RunningBalancerSnapshot>),
    Stop,
    CopyToClipboard(String),
    OpenUrl(String),
}

pub enum Action {
    None,
    Stop,
    CopyToClipboard(String),
    OpenUrl(String),
}

impl RunningBalancerData {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::SnapshotUpdated(snapshot) => {
                self.snapshot = *snapshot;

                Action::None
            }
            Message::Stop => {
                self.stopping = true;

                Action::Stop
            }
            Message::CopyToClipboard(content) => Action::CopyToClipboard(content),
            Message::OpenUrl(url) => Action::OpenUrl(url),
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use anyhow::Result;
    use paddler_types::agent_desired_model::AgentDesiredModel;
    use paddler_types::balancer_desired_state::BalancerDesiredState;

    use super::Action;
    use super::Message;
    use super::RunningBalancerData;
    use super::RunningBalancerSnapshot;

    fn fresh_data() -> RunningBalancerData {
        RunningBalancerData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            snapshot: RunningBalancerSnapshot::default(),
            stopping: false,
            web_admin_panel_address: None,
        }
    }

    #[test]
    fn snapshot_updated_replaces_snapshot_and_returns_none() -> Result<()> {
        let mut data = fresh_data();
        let new_snapshot = RunningBalancerSnapshot {
            balancer_desired_state: BalancerDesiredState {
                model: AgentDesiredModel::LocalToAgent("/some/model.gguf".to_owned()),
                ..BalancerDesiredState::default()
            },
            ..RunningBalancerSnapshot::default()
        };

        let action = data.update(Message::SnapshotUpdated(Box::new(new_snapshot)));

        assert!(matches!(action, Action::None));
        assert!(matches!(
            &data.snapshot.balancer_desired_state.model,
            AgentDesiredModel::LocalToAgent(path) if path == "/some/model.gguf"
        ));

        Ok(())
    }

    #[test]
    fn stop_message_sets_stopping_flag_and_returns_stop_action() -> Result<()> {
        let mut data = fresh_data();

        let action = data.update(Message::Stop);

        assert!(matches!(action, Action::Stop));
        assert!(
            data.stopping,
            "expected stopping flag to flip to true after Stop"
        );

        Ok(())
    }

    #[test]
    fn copy_to_clipboard_message_forwards_content_as_action() -> Result<()> {
        let mut data = fresh_data();

        let action = data.update(Message::CopyToClipboard("address-value".to_owned()));

        assert!(matches!(
            action,
            Action::CopyToClipboard(content) if content == "address-value"
        ));

        Ok(())
    }

    #[test]
    fn open_url_message_forwards_url_as_action() -> Result<()> {
        let mut data = fresh_data();

        let action = data.update(Message::OpenUrl("http://example.test".to_owned()));

        assert!(matches!(
            action,
            Action::OpenUrl(url) if url == "http://example.test"
        ));

        Ok(())
    }
}
