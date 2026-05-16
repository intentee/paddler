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
    use anyhow::Result;
    use anyhow::bail;
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

        match data.update(Message::SnapshotUpdated(Box::new(new_snapshot))) {
            Action::None => {}
            _ => bail!("expected Action::None for SnapshotUpdated"),
        }

        match &data.snapshot.balancer_desired_state.model {
            AgentDesiredModel::LocalToAgent(path) if path == "/some/model.gguf" => Ok(()),
            other => bail!("expected snapshot's model to be replaced, got {other:?}"),
        }
    }

    #[test]
    fn stop_message_sets_stopping_flag_and_returns_stop_action() -> Result<()> {
        let mut data = fresh_data();

        match data.update(Message::Stop) {
            Action::Stop => {}
            _ => bail!("expected Action::Stop"),
        }

        if !data.stopping {
            bail!("expected stopping flag to flip to true after Stop");
        }

        Ok(())
    }

    #[test]
    fn copy_to_clipboard_message_forwards_content_as_action() -> Result<()> {
        let mut data = fresh_data();

        match data.update(Message::CopyToClipboard("address-value".to_owned())) {
            Action::CopyToClipboard(content) if content == "address-value" => Ok(()),
            _ => bail!("expected Action::CopyToClipboard with the forwarded content"),
        }
    }

    #[test]
    fn open_url_message_forwards_url_as_action() -> Result<()> {
        let mut data = fresh_data();

        match data.update(Message::OpenUrl("http://example.test".to_owned())) {
            Action::OpenUrl(url) if url == "http://example.test" => Ok(()),
            _ => bail!("expected Action::OpenUrl with the forwarded url"),
        }
    }
}
