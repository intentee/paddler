#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_client::Message;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_504_when_inference_item_timeout_is_zero() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let cluster = start_subprocess_cluster(SubprocessClusterParams {
        agent_count: 1,
        slots_per_agent: 2,
        inference_item_timeout: Duration::ZERO,
        wait_for_slots_ready: true,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..SubprocessClusterParams::default()
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await?;

    let message = stream
        .next()
        .await
        .context("inference stream must yield a message")??;

    match message {
        Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 504);
        }
        Message::Response(_) => {
            anyhow::bail!("expected timeout error, got success");
        }
    }

    cluster.shutdown().await?;

    Ok(())
}
