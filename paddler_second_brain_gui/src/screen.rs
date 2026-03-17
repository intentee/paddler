use statum::machine;
use statum::state;
use statum::transition;

use crate::running_cluster_data::RunningClusterData;
use crate::start_cluster_config_data::StartClusterConfigData;
use crate::starting_cluster_data::StartingClusterData;

#[state]
pub enum ScreenState {
    Home,
    StartClusterConfig(StartClusterConfigData),
    StartingCluster(StartingClusterData),
    RunningCluster(RunningClusterData),
    StoppingCluster,
}

#[machine]
pub struct Screen<ScreenState> {}

#[transition]
impl Screen<Home> {
    pub fn start_cluster(self) -> Screen<StartClusterConfig> {
        self.transition_with(StartClusterConfigData::default())
    }
}

#[transition]
impl Screen<StartClusterConfig> {
    pub fn cancel(self) -> Screen<Home> {
        self.transition()
    }

    pub fn confirm(self) -> Screen<StartingCluster> {
        self.transition_map(|config_data| StartingClusterData {
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
    pub fn cluster_started(self, cluster_address: String) -> Screen<RunningCluster> {
        self.transition_map(|starting_data| RunningClusterData {
            cluster_address,
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
