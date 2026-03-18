use statum::machine;
use statum::state;
use statum::transition;

use crate::agent_running_data::AgentRunningData;
use crate::join_cluster_config_data::JoinClusterConfigData;
use crate::network_interface_address::NetworkInterfaceAddress;
use crate::running_cluster_data::RunningClusterData;
use crate::start_cluster_config_data::StartClusterConfigData;
use crate::starting_cluster_data::StartingClusterData;

#[state]
pub enum ScreenState {
    AgentRunning(AgentRunningData),
    Home,
    JoinClusterConfig(JoinClusterConfigData),
    StartClusterConfig(StartClusterConfigData),
    StartingCluster(StartingClusterData),
    RunningCluster(RunningClusterData),
    StoppingCluster,
}

#[machine]
pub struct Screen<ScreenState> {}

#[transition]
impl Screen<Home> {
    pub fn join_cluster(self) -> Screen<JoinClusterConfig> {
        self.transition_with(JoinClusterConfigData::default())
    }

    pub fn start_cluster(self) -> Screen<StartClusterConfig> {
        self.transition_with(StartClusterConfigData::default())
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

    pub fn confirm(
        self,
        network_interfaces: Vec<NetworkInterfaceAddress>,
        management_port: u16,
    ) -> Screen<StartingCluster> {
        self.transition_map(|config_data| StartingClusterData {
            network_interfaces,
            management_port,
            selected_model_name: config_data
                .selected_model
                .map(|preset| preset.display_name)
                .unwrap_or_default(),
            run_agent_locally: config_data.run_agent_locally,
        })
    }
}

#[transition]
impl Screen<StartingCluster> {
    pub fn cluster_started(self) -> Screen<RunningCluster> {
        self.transition_map(|starting_data| RunningClusterData {
            agent_count: 0,
            network_interfaces: starting_data.network_interfaces,
            management_port: starting_data.management_port,
            selected_model_name: starting_data.selected_model_name,
            run_agent_locally: starting_data.run_agent_locally,
        })
    }

    pub fn cluster_failed(self) -> Screen<Home> {
        self.transition()
    }
}

#[transition]
impl Screen<RunningCluster> {
    pub fn dismiss(self) -> Screen<Home> {
        self.transition()
    }

    pub fn stop(self) -> Screen<StoppingCluster> {
        self.transition()
    }

    pub fn cluster_failed(self) -> Screen<Home> {
        self.transition()
    }
}

#[transition]
impl Screen<StoppingCluster> {
    pub fn cluster_stopped(self) -> Screen<Home> {
        self.transition()
    }

    pub fn cluster_failed(self) -> Screen<Home> {
        self.transition()
    }
}
