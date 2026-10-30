use std::path::PathBuf;

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;

use crate::decision_backbone::DecisionBackbone;
use crate::decision_cluster_params::DecisionClusterParams;
use crate::decision_desired_state::DecisionDesiredState;

const KEV_27B_GPU_LAYERS_FITTING_BESIDE_EVERY_CONTEXT: i32 = 55;

#[derive(Clone, Copy)]
pub enum KevCheckpoint {
    Kev0_8b,
    Kev27b,
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
            n_gpu_layers: self.gpu_layers(),
            pointer_head: self.target_path("pointer_head.gguf"),
            ..DecisionDesiredState::default()
        }
    }

    const fn gpu_layers(self) -> i32 {
        match self {
            Self::Kev0_8b => ALL_GPU_LAYERS,
            Self::Kev27b => KEV_27B_GPU_LAYERS_FITTING_BESIDE_EVERY_CONTEXT,
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
            Self::Kev27b => "kev_27b",
        }
    }
}
