use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;

use crate::agent_config::AgentConfig;
use crate::cluster_handle::ClusterHandle;
use crate::cluster_params::ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_6_35b_a3b::qwen3_6_35b_a3b;
use crate::model_card::qwen3_6_35b_a3b_mmproj::qwen3_6_35b_a3b_mmproj;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_qwen3_6_and_mmproj(
    agents: Vec<AgentConfig>,
) -> Result<ClusterHandle> {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = qwen3_6_35b_a3b();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = qwen3_6_35b_a3b_mmproj();

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
