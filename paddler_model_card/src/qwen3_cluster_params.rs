use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::model_card::ModelCard;
use crate::qwen3_0_6b::qwen3_0_6b;

#[must_use]
pub fn qwen3_cluster_params(agents: Vec<AgentConfig>) -> ClusterParams {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    ClusterParams {
        agents,
        desired_state: DesiredStateInit::set(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::deterministic()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
    }
}
