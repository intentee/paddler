use clap::ValueEnum;

use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_model_card_fixture::open_jev::open_jev;

use crate::agent_config::AgentConfig;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;
use crate::decision_cluster_params::DecisionClusterParams;

const TEXT_GENERATION_SLOTS: u16 = 1;

#[derive(Clone, Copy, ValueEnum)]
pub enum TestClusterPreset {
    #[value(name = "kev-0-8b")]
    Kev0_8b,
    #[value(name = "open-jev")]
    OpenJev,
    #[value(name = "qwen3-0-6b")]
    Qwen3_0_6b,
}

impl TestClusterPreset {
    #[must_use]
    pub fn cluster_params(self) -> ClusterParams {
        match self {
            Self::Kev0_8b => DecisionClusterParams::default().into_cluster_params(),
            Self::OpenJev => DecisionClusterParams {
                desired_state: DecisionDesiredState {
                    model_card: open_jev(),
                    ..DecisionDesiredState::default()
                },
                ..DecisionClusterParams::default()
            }
            .into_cluster_params(),
            Self::Qwen3_0_6b => ClusterParams {
                agents: AgentConfig::uniform(1, TEXT_GENERATION_SLOTS),
                desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
                ..ClusterParams::default()
            },
        }
    }
}
