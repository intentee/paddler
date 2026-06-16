use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::in_process_cluster_backend::InProcessClusterBackend;
use crate::model_card::ModelCard;
use crate::model_card::qwen2_5_vl_3b::qwen2_5_vl_3b;
use crate::model_card::qwen2_5_vl_3b_mmproj::qwen2_5_vl_3b_mmproj;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;

pub async fn start_cluster_with_qwen2_5_vl(agents: Vec<AgentConfig>) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = qwen2_5_vl_3b();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = qwen2_5_vl_3b_mmproj();

    Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents,
            desired_state: Some(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    n_gpu_layers: gpu_layer_count,
                    ..InferenceParameters::deterministic()
                },
                model: AgentDesiredModel::HuggingFace(primary_reference),
                multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
        },
    )
    .await
}
