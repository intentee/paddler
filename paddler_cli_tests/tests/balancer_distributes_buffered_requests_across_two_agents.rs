#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_cli_tests::subprocess_cluster_backend::SubprocessClusterBackend;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_buffered_requests_across_two_agents() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let cluster = Cluster::start(
        &SubprocessClusterBackend::new(env!("CARGO_BIN_EXE_paddler_cluster_node"))
            .with_service_config(BalancerServiceConfig {
                buffered_request_timeout: Duration::from_mins(2),
                max_buffered_requests: 10,
                ..Default::default()
            }),
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
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    n_gpu_layers: gpu_layer_count,
                    ..InferenceParameters::default()
                },
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
        },
    )
    .await?;

    let mut streams = Vec::new();

    for _ in 0..5 {
        let stream = cluster
            .inference_client
            .http()
            .continue_from_raw_prompt(&ContinueFromRawPromptParams {
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
