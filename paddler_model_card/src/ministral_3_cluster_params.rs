use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::ministral_3_14b_reasoning::ministral_3_14b_reasoning;
use crate::model_card::ModelCard;

pub struct Ministral3ClusterParams {
    pub agents: Vec<AgentConfig>,
    pub deterministic_sampling: bool,
}

impl Ministral3ClusterParams {
    #[must_use]
    pub fn into_cluster_params(self) -> ClusterParams {
        let ModelCard {
            gpu_layer_count,
            reference,
        } = ministral_3_14b_reasoning();

        let inference_parameters = if self.deterministic_sampling {
            InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::deterministic()
            }
        } else {
            InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::default()
            }
        };

        ClusterParams {
            agents: self.agents,
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters,
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
        }
    }
}

impl Default for Ministral3ClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 1),
            deterministic_sampling: false,
        }
    }
}
