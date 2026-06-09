#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_completes_buffered_request_after_agent_joins() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        buffered_request_timeout: Duration::from_mins(2),
        max_buffered_requests: 1,
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
    })
    .await?;

    let mut stream = cluster
        .continue_from_raw_prompt_stream(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await?;

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .context("balancer should buffer the request before any agent joins")?;

    cluster.spawn_additional_agent(&AgentConfig {
        name: "buffered-agent".to_owned(),
        slot_count: 4,
    })?;

    let message = stream
        .next()
        .await
        .context("inference stream must yield a message after agent joins")??;

    match message {
        Message::Response(_) => {}
        Message::Error(envelope) => {
            anyhow::bail!(
                "expected a successful response, got error code {}: {}",
                envelope.error.code,
                envelope.error.description
            );
        }
    }

    cluster.shutdown().await?;

    Ok(())
}
