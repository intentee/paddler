#![cfg(feature = "tests_that_use_docker")]

use std::env;

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_testcontainer::container_cluster_backend::ContainerClusterBackend;

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn model_reference() -> HuggingFaceModelReference {
    HuggingFaceModelReference {
        filename: env_or(
            "PADDLER_TESTCONTAINER_MODEL_FILENAME",
            "Qwen3-0.6B-Q8_0.gguf",
        ),
        repo_id: env_or("PADDLER_TESTCONTAINER_MODEL_REPO", "Qwen/Qwen3-0.6B-GGUF"),
        revision: env_or("PADDLER_TESTCONTAINER_MODEL_REVISION", "main"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn container_cluster_runs_inference() -> Result<()> {
    let cluster = Cluster::start(
        &ContainerClusterBackend,
        ClusterParams {
            agents: vec![AgentConfig::single(1)],
            desired_state: Some(BalancerDesiredState {
                inference_parameters: InferenceParameters {
                    n_gpu_layers: 0,
                    ..InferenceParameters::default()
                },
                model: AgentDesiredModel::HuggingFace(model_reference()),
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
