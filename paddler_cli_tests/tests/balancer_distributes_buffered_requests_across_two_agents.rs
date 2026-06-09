#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_cli_tests::model_card::ModelCard;
use paddler_cli_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_buffered_requests_across_two_agents() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: vec![
                AgentConfig {
                    name: "distributed-agent-0".to_owned(),
                    slot_count: 2,
                },
                AgentConfig {
                    name: "distributed-agent-1".to_owned(),
                    slot_count: 2,
                },
            ],
            wait_for_slots_ready: true,
            buffered_request_timeout: Duration::from_mins(2),
            max_buffered_requests: 10,
            desired_state: Some(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    n_gpu_layers: gpu_layer_count,
                    ..InferenceParameters::default()
                },
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
            ..ClusterParams::default()
        },
    )
    .await?;

    let mut streams = Vec::new();

    for _ in 0..5 {
        let stream = cluster
            .continue_from_raw_prompt_stream(&ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 10,
                raw_prompt: "Hello".to_owned(),
            })
            .await?;

        streams.push(stream);
    }

    let mut successful_responses = 0;

    for mut stream in streams {
        if let Some(item) = stream.next().await {
            match item? {
                Message::Response(_) => successful_responses += 1,
                Message::Error(envelope) => {
                    anyhow::bail!(
                        "expected success, got error {}: {}",
                        envelope.error.code,
                        envelope.error.description
                    );
                }
            }
        }
    }

    assert_eq!(successful_responses, 5);

    cluster.shutdown().await?;

    Ok(())
}
