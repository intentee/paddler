use clap::ValueEnum;

use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::kev_checkpoint::KevCheckpoint;
use crate::qwen3_desired_state::qwen3_desired_state;

const TEXT_GENERATION_SLOTS: u16 = 1;

#[derive(Clone, Copy, ValueEnum)]
pub enum TestClusterPreset {
    #[value(name = "kev-0-8b")]
    Kev0_8b,
    #[value(name = "qwen3-0-6b")]
    Qwen3_0_6b,
}

impl TestClusterPreset {
    pub fn cluster_params(self) -> Result<ClusterParams, ModelSourceError> {
        match self {
            Self::Kev0_8b => KevCheckpoint::Kev0_8b
                .decision_cluster_params()
                .into_cluster_params(),
            Self::Qwen3_0_6b => Ok(ClusterParams {
                agents: AgentConfig::uniform(1, TEXT_GENERATION_SLOTS),
                desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
                ..ClusterParams::default()
            }),
        }
    }
}
