use std::sync::Arc;

use statum::machine;
use statum::state;
use statum::transition;
use tokio_util::sync::CancellationToken;

use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_balancer_runner::balancer_runner_error::BalancerRunnerError;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_connection::BalancerConnection;
use paddler_service_thread::service_thread_error::ServiceThreadError;

use crate::agent_running_data::AgentRunningData;
use crate::detect_network_interfaces::detect_network_interfaces;
use crate::home_data::HomeData;
use crate::join_balancer_form_data::JoinBalancerFormData;
use crate::runner_failure::RunnerFailure;
use crate::running_balancer_data::RunningBalancerData;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;
use crate::start_balancer_form_data::StartBalancerFormData;

#[state]
pub enum ScreenState {
    AgentRunning(AgentRunningData),
    Home(HomeData),
    JoinBalancerForm(JoinBalancerFormData),
    StartBalancerForm(StartBalancerFormData),
    RunningBalancer(RunningBalancerData),
}

#[machine]
pub struct Screen<ScreenState> {}

#[transition]
impl Screen<Home> {
    #[must_use]
    pub fn join_balancer(self) -> Screen<JoinBalancerForm> {
        self.transition_with(JoinBalancerFormData::default())
    }

    #[must_use]
    pub fn start_balancer(self) -> Screen<StartBalancerForm> {
        self.transition_with(StartBalancerFormData::suggesting_addresses_on(
            detect_network_interfaces()
                .first()
                .map(|interface| interface.ip_address),
        ))
    }
}

#[transition]
impl Screen<JoinBalancerForm> {
    #[must_use]
    pub fn cancel(self) -> Screen<Home> {
        self.transition_with(HomeData::Welcome)
    }

    #[must_use]
    pub fn connect(self, cancellation_token: CancellationToken) -> Screen<AgentRunning> {
        self.transition_map(|form_data: JoinBalancerFormData| {
            let name = form_data.entered_agent_name();

            AgentRunningData {
                balancer_address: form_data.balancer_address,
                balancer_connection: BalancerConnection::Connecting,
                cancellation_token,
                name,
                slots_processing: 0,
                status: AgentStatus::default(),
            }
        })
    }
}

#[transition]
impl Screen<AgentRunning> {
    #[must_use]
    pub fn disconnect(self) -> Screen<Home> {
        self.transition_with(HomeData::Welcome)
    }

    #[must_use]
    pub fn agent_failed(self, service_thread_error: Arc<ServiceThreadError>) -> Screen<Home> {
        self.transition_with(HomeData::ReturnedAfterFailure(RunnerFailure::Agent(
            service_thread_error,
        )))
    }
}

#[transition]
impl Screen<StartBalancerForm> {
    #[must_use]
    pub fn cancel(self) -> Screen<Home> {
        self.transition_with(HomeData::Welcome)
    }

    #[must_use]
    pub fn balancer_started(
        self,
        addresses: BalancerAddresses,
        cancellation_token: CancellationToken,
        snapshot: Box<RunningBalancerSnapshot>,
    ) -> Screen<RunningBalancer> {
        self.transition_with(RunningBalancerData {
            addresses,
            cancellation_token,
            snapshot,
            stopping: false,
        })
    }

    #[must_use]
    pub fn balancer_failed(self, balancer_runner_error: Arc<BalancerRunnerError>) -> Screen<Home> {
        self.transition_with(HomeData::ReturnedAfterFailure(RunnerFailure::Balancer(
            balancer_runner_error,
        )))
    }
}

#[transition]
impl Screen<RunningBalancer> {
    #[must_use]
    pub fn balancer_stopped(self) -> Screen<Home> {
        self.transition_with(HomeData::Welcome)
    }

    #[must_use]
    pub fn balancer_failed(self, balancer_runner_error: Arc<BalancerRunnerError>) -> Screen<Home> {
        self.transition_with(HomeData::ReturnedAfterFailure(RunnerFailure::Balancer(
            balancer_runner_error,
        )))
    }
}
