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
    use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::Action;
    use super::Message;
    use crate::running_balancer_data::RunningBalancerData;
    use crate::running_balancer_snapshot::RunningBalancerSnapshot;

    #[test]
    fn stopping_marks_the_balancer_as_stopping() {
        let mut balancer = RunningBalancerData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            snapshot: RunningBalancerSnapshot {
                agent_snapshots: Vec::new(),
                balancer_applicable_state: BalancerApplicableState::from(
                    BalancerDesiredState::default(),
                ),
                balancer_desired_state: BalancerDesiredState::default(),
            },
            stopping: false,
            web_admin_panel_address: None,
        };

        let action = balancer.update(Message::Stop);

        assert!(matches!(action, Action::Stop));
        assert!(balancer.stopping);
    }
}
