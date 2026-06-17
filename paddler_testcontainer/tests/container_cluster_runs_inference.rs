#![cfg(feature = "tests_that_use_docker")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_testcontainer::container_cluster_backend::ContainerClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn container_cluster_runs_inference() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let cluster = Cluster::start(
        &ContainerClusterBackend::default(),
        ClusterParams {
            agents: vec![AgentConfig::single(1)],
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                inference_parameters: InferenceParameters {
                    n_gpu_layers: gpu_layer_count,
                    ..InferenceParameters::default()
                },
                model: AgentDesiredModel::HuggingFace(reference),
                ..BalancerDesiredState::default()
            }),
            wait_for_slots_ready: true,
        },
    )
    .await?;

    let collected = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt_collected(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 16,
            raw_prompt: "The capital of France is".to_owned(),
        })
        .await?;

    assert!(
        !collected.text.is_empty(),
        "inference through the container cluster must produce tokens"
    );

    cluster.shutdown().await?;

    Ok(())
}
