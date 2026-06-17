use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::in_process_cluster_backend::InProcessClusterBackend;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_model_card::gemma_4_e4b_it::gemma_4_e4b_it;
use paddler_model_card::gemma_4_e4b_it_mmproj::gemma_4_e4b_it_mmproj;
use paddler_model_card::model_card::ModelCard;

pub async fn start_cluster_with_gemma_4_and_mmproj(agents: Vec<AgentConfig>) -> Result<Cluster> {
    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = gemma_4_e4b_it();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = gemma_4_e4b_it_mmproj();

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
                multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
        },
    )
    .await
}
