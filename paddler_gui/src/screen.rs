use std::collections::BTreeSet;

use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
use statum::machine;
use statum::state;
use statum::transition;

use crate::agent_running_data::AgentRunningData;
use crate::detect_network_interfaces::detect_network_interfaces;
use crate::home_data::HomeData;
use crate::join_balancer_config_data::JoinBalancerConfigData;
use crate::running_balancer_data::RunningBalancerData;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;
use crate::start_balancer_config_data::StartBalancerConfigData;

#[state]
pub enum ScreenState {
    AgentRunning(AgentRunningData),
    Home(HomeData),
    JoinBalancerConfig(JoinBalancerConfigData),
    StartBalancerConfig(StartBalancerConfigData),
    RunningBalancer(RunningBalancerData),
}

#[machine]
pub struct Screen<ScreenState> {}

#[transition]
impl Screen<Home> {
    pub fn join_balancer(self) -> Screen<JoinBalancerConfig> {
        self.transition_with(JoinBalancerConfigData::default())
    }

    pub fn start_balancer(self) -> Screen<StartBalancerConfig> {
        let suggested_address = detect_network_interfaces()
            .first()
            .map(|interface| interface.ip_address.to_string())
            .unwrap_or_default();

        self.transition_with(StartBalancerConfigData {
            add_model_later: false,
            balancer_address: format!("{suggested_address}:8060"),
            balancer_address_error: None,
            inference_address: format!("{suggested_address}:8061"),
            inference_address_error: None,
            model_error: None,
            selected_model: None,
            starting: false,
        })
    }
}

#[transition]
impl Screen<JoinBalancerConfig> {
    pub fn cancel(self) -> Screen<Home> {
        self.transition_with(HomeData { error: None })
    }

    pub fn connect(self) -> Screen<AgentRunning> {
        self.transition_map(|config_data: JoinBalancerConfigData| {
            let name = if config_data.agent_name.is_empty() {
                None
            } else {
                Some(config_data.agent_name)
            };

            AgentRunningData {
                balancer_address: config_data.balancer_address,
                connected: false,
                snapshot: AgentControllerSnapshot {
                    desired_slots_total: 0,
                    download_current: 0,
                    download_filename: None,
                    download_total: 0,
                    id: String::new(),
                    issues: BTreeSet::new(),
                    model_path: None,
                    name,
                    slots_processing: 0,
                    slots_total: 0,
                    state_application_status: AgentStateApplicationStatus::Fresh,
                    uses_chat_template_override: false,
                },
            }
        })
    }
}

#[transition]
impl Screen<AgentRunning> {
    pub fn disconnect(self) -> Screen<Home> {
        self.transition_with(HomeData { error: None })
    }

    pub fn agent_failed(self, error: String) -> Screen<Home> {
        self.transition_with(HomeData { error: Some(error) })
    }
}

#[transition]
impl Screen<StartBalancerConfig> {
    pub fn cancel(self) -> Screen<Home> {
        self.transition_with(HomeData { error: None })
    }

    pub fn balancer_started(self) -> Screen<RunningBalancer> {
        self.transition_map(|config_data: StartBalancerConfigData| RunningBalancerData {
            balancer_address: config_data.balancer_address,
            snapshot: RunningBalancerSnapshot::default(),
            stopping: false,
        })
    }

    pub fn balancer_failed(self, error: String) -> Screen<Home> {
        self.transition_with(HomeData { error: Some(error) })
    }
}

#[transition]
impl Screen<RunningBalancer> {
    pub fn balancer_stopped(self) -> Screen<Home> {
        self.transition_with(HomeData { error: None })
    }

    pub fn balancer_failed(self, error: String) -> Screen<Home> {
        self.transition_with(HomeData { error: Some(error) })
    }
}
