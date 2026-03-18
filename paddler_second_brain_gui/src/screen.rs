use statum::machine;
use statum::state;
use statum::transition;

use crate::agent_running_data::AgentRunningData;
use crate::detect_network_interfaces::detect_network_interfaces;
use crate::join_cluster_config_data::JoinClusterConfigData;
use crate::running_cluster_data::RunningClusterData;
use crate::start_cluster_config_data::StartClusterConfigData;

#[state]
pub enum ScreenState {
    AgentRunning(AgentRunningData),
    Home,
    JoinClusterConfig(JoinClusterConfigData),
    StartClusterConfig(StartClusterConfigData),
    RunningCluster(RunningClusterData),
}

#[machine]
pub struct Screen<ScreenState> {}

#[transition]
impl Screen<Home> {
    pub fn join_cluster(self) -> Screen<JoinClusterConfig> {
        self.transition_with(JoinClusterConfigData::default())
    }

    pub fn start_cluster(self) -> Screen<StartClusterConfig> {
        let suggested_address = detect_network_interfaces()
            .first()
            .map(|interface| interface.ip_address.to_string())
            .unwrap_or_default();

        self.transition_with(StartClusterConfigData {
            balancer_address: format!("{suggested_address}:8060"),
            error: None,
            inference_address: format!("{suggested_address}:8061"),
            selected_model: None,
            starting: false,
        })
    }
}

#[transition]
impl Screen<JoinClusterConfig> {
    pub fn cancel(self) -> Screen<Home> {
        self.transition()
    }

    pub fn connect(self) -> Screen<AgentRunning> {
        self.transition_with(AgentRunningData { status: None })
    }
}

#[transition]
impl Screen<AgentRunning> {
    pub fn disconnect(self) -> Screen<Home> {
        self.transition()
    }

    pub fn agent_failed(self) -> Screen<Home> {
        self.transition()
    }
}

#[transition]
impl Screen<StartClusterConfig> {
    pub fn cancel(self) -> Screen<Home> {
        self.transition()
    }

    pub fn cluster_started(self) -> Screen<RunningCluster> {
        self.transition_map(|config_data| RunningClusterData {
            agent_count: 0,
            cluster_address: config_data.balancer_address.clone(),
            stopping: false,
        })
    }

    pub fn cluster_failed(self) -> Screen<Home> {
        self.transition()
    }
}

#[transition]
impl Screen<RunningCluster> {
    pub fn cluster_stopped(self) -> Screen<Home> {
        self.transition()
    }

    pub fn cluster_failed(self) -> Screen<Home> {
        self.transition()
    }
}
