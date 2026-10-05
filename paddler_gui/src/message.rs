use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_bootstrap::bootstrap_error::BootstrapError;

use crate::agent_running_message::AgentRunningMessage;
use crate::home_message::HomeMessage;
use crate::join_balancer_form_message::JoinBalancerFormMessage;
use crate::running_balancer_message::RunningBalancerMessage;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;
use crate::start_balancer_form_message::StartBalancerFormMessage;

#[derive(Debug, Clone)]
pub enum Message {
    Home(HomeMessage),
    StartBalancerForm(StartBalancerFormMessage),
    JoinBalancerForm(JoinBalancerFormMessage),
    RunningBalancer(RunningBalancerMessage),
    AgentRunning(AgentRunningMessage),
    BalancerStarted {
        addresses: BalancerAddresses,
        cancellation_token: CancellationToken,
        snapshot: Box<RunningBalancerSnapshot>,
    },
    BalancerStopped,
    BalancerFailed(Arc<BootstrapError>),
    AgentStopped,
    AgentFailed(Arc<BootstrapError>),
    IcedEventLoopReady,
    Quit,
    TabPressed {
        shift: bool,
    },
}
