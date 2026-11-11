use std::path::PathBuf;

use crate::decision_backbone::DecisionBackbone;
use crate::decision_cluster_params::DecisionClusterParams;
use crate::decision_desired_state::DecisionDesiredState;

#[derive(Clone, Copy)]
pub enum KevCheckpoint {
    Kev0_8b,
}

impl KevCheckpoint {
    #[must_use]
    pub fn decision_cluster_params(self) -> DecisionClusterParams {
        DecisionClusterParams {
            desired_state: self.decision_desired_state(),
            ..DecisionClusterParams::default()
        }
    }

    #[must_use]
    pub fn decision_desired_state(self) -> DecisionDesiredState {
        DecisionDesiredState {
            backbone: DecisionBackbone::LocalFile(self.target_path("model.gguf")),
            pointer_head: self.target_path("pointer_head.gguf"),
            ..DecisionDesiredState::default()
        }
    }

    #[must_use]
    pub fn target_path(self, artifact_file_name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .with_file_name("target")
            .join(self.target_directory_name())
            .join(artifact_file_name)
    }

    const fn target_directory_name(self) -> &'static str {
        match self {
            Self::Kev0_8b => "kev_0_8b",
        }
    }
}
