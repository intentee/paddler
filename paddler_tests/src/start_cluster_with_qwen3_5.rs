use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_params::ClusterParams;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use crate::model_card::qwen3_5_0_8b_mmproj::qwen3_5_0_8b_mmproj;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_qwen3_5(
    agents: Vec<AgentConfig>,
    with_mmproj: bool,
) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = qwen3_5_0_8b();

    let multimodal_projection = if with_mmproj {
        let ModelCard {
            reference: mmproj_reference,
            ..
        } = qwen3_5_0_8b_mmproj();

        AgentDesiredModel::HuggingFace(mmproj_reference)
    } else {
        AgentDesiredModel::None
    };

    start_cluster(ClusterParams {
        agents,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
