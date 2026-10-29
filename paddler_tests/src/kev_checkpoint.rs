use std::path::PathBuf;

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

const KEV_27B_GPU_LAYERS_FITTING_BESIDE_EVERY_CONTEXT: i32 = 55;

#[derive(Clone, Copy)]
pub enum KevCheckpoint {
    Kev0_8b,
    Kev27b,
}

impl KevCheckpoint {
    pub fn cluster_params(self) -> Result<ClusterParams, ModelSourceError> {
        self.desired_state().map(|desired_state| ClusterParams {
            agents: AgentConfig::uniform(1, DECISION_SLOTS_MINIMUM.get()),
            desired_state: ClusterDesiredState::Apply(Box::new(desired_state)),
            ..ClusterParams::default()
        })
    }

    pub fn desired_state(self) -> Result<BalancerDesiredState, ModelSourceError> {
        agent_desired_model_at(&self.target_path("pointer_head.gguf")).and_then(|pointer_head| {
            agent_desired_model_at(&self.target_path("model.gguf")).map(|model| {
                BalancerDesiredState {
                    decision: DecisionSettings { pointer_head },
                    model,
                    model_runtime_parameters: ModelRuntimeParameters {
                        n_gpu_layers: self.gpu_layers(),
                        ..ModelRuntimeParameters::default()
                    },
                    ..BalancerDesiredState::default()
                }
            })
        })
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
