use tokio_util::sync::CancellationToken;

use paddler_balancer::balancer_addresses::BalancerAddresses;

use crate::running_balancer_action::RunningBalancerAction;
use crate::running_balancer_message::RunningBalancerMessage;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;

pub struct RunningBalancerData {
    pub addresses: BalancerAddresses,
    pub cancellation_token: CancellationToken,
    pub snapshot: Box<RunningBalancerSnapshot>,
    pub stopping: bool,
}

impl RunningBalancerData {
    pub fn update(&mut self, message: RunningBalancerMessage) -> RunningBalancerAction {
        match message {
            RunningBalancerMessage::SnapshotUpdated(snapshot) => {
                self.snapshot = snapshot;

                RunningBalancerAction::None
            }
            RunningBalancerMessage::Stop => {
                self.cancellation_token.cancel();
                self.stopping = true;

                RunningBalancerAction::None
            }
            RunningBalancerMessage::CopyToClipboard(content) => {
                RunningBalancerAction::CopyToClipboard(content)
            }
            RunningBalancerMessage::OpenUrl(url) => RunningBalancerAction::OpenUrl(url),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;

    use tokio_util::sync::CancellationToken;

    use paddler_balancer::balancer_addresses::BalancerAddresses;
    use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::RunningBalancerData;
    use crate::running_balancer_action::RunningBalancerAction;
    use crate::running_balancer_message::RunningBalancerMessage;
    use crate::running_balancer_snapshot::RunningBalancerSnapshot;

    fn running_balancer() -> RunningBalancerData {
        RunningBalancerData {
            addresses: BalancerAddresses {
                compat_openai: None,
                compat_typesafe: None,
                inference: SocketAddr::from((Ipv4Addr::LOCALHOST, 8061)),
                management: SocketAddr::from((Ipv4Addr::LOCALHOST, 8060)),
                web_admin_panel: None,
            },
            cancellation_token: CancellationToken::new(),
            snapshot: Box::new(RunningBalancerSnapshot {
                agent_snapshots: Vec::new(),
                balancer_applicable_state: BalancerApplicableState::from(
                    BalancerDesiredState::default(),
                ),
                balancer_desired_state: BalancerDesiredState::default(),
            }),
            stopping: false,
        }
    }

    #[test]
    fn stopping_cancels_the_running_balancer() {
        let mut balancer = running_balancer();

        let action = balancer.update(RunningBalancerMessage::Stop);

        assert_eq!(action, RunningBalancerAction::None);
        assert!(balancer.cancellation_token.is_cancelled());
        assert!(balancer.stopping);
    }

    #[test]
    fn following_a_link_asks_to_open_it() {
        let mut balancer = running_balancer();

        assert_eq!(
            balancer.update(RunningBalancerMessage::OpenUrl(
                "http://127.0.0.1:8062".to_owned()
            )),
            RunningBalancerAction::OpenUrl("http://127.0.0.1:8062".to_owned())
        );
    }
}
