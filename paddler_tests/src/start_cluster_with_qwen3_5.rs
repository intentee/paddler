use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::in_process_cluster_backend::InProcessClusterBackend;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_model_card::qwen3_5_0_8b_mmproj::qwen3_5_0_8b_mmproj;

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

    Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents,
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    n_gpu_layers: gpu_layer_count,
                    ..InferenceParameters::deterministic()
                },
                model: AgentDesiredModel::HuggingFace(primary_reference),
                multimodal_projection,
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
        },
    )
    .await
}
