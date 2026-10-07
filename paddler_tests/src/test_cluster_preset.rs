use clap::ValueEnum;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::kev_0_8b_desired_state::kev_0_8b_desired_state;
use crate::qwen3_desired_state::qwen3_desired_state;

const DECISION_SLOTS: u16 = 2;
const TEXT_GENERATION_SLOTS: u16 = 1;

#[derive(Clone, Copy, ValueEnum)]
pub enum TestClusterPreset {
    #[value(name = "kev-0-8b")]
    Kev0_8b,
    #[value(name = "qwen3-0-6b")]
    Qwen3_0_6b,
}

impl TestClusterPreset {
    #[must_use]
    pub fn cluster_params(self) -> ClusterParams {
        match self {
            Self::Kev0_8b => ClusterParams {
                agents: AgentConfig::uniform(1, DECISION_SLOTS),
                desired_state: ClusterDesiredState::Apply(Box::new(kev_0_8b_desired_state())),
                ..ClusterParams::default()
            },
            Self::Qwen3_0_6b => ClusterParams {
                agents: AgentConfig::uniform(1, TEXT_GENERATION_SLOTS),
                desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
                ..ClusterParams::default()
            },
        }
    }
}
