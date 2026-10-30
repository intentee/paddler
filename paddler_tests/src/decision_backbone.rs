use std::path::PathBuf;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_test_cluster_harness::model_card::ModelCard;

pub enum DecisionBackbone {
    HuggingFace(ModelCard),
    LocalFile(PathBuf),
}

impl DecisionBackbone {
    pub fn into_desired_state(self) -> Result<BalancerDesiredState, ModelSourceError> {
        match self {
            Self::HuggingFace(model_card) => Ok(model_card.into_desired_state()),
            Self::LocalFile(model_path) => Ok(BalancerDesiredState {
                model: agent_desired_model_at(&model_path)?,
                ..BalancerDesiredState::default()
            }),
        }
    }
}
