#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::buffered_requests_status::assert_count::assert_count;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_client::Message;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_completes_buffered_request_after_agent_joins() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = start_subprocess_cluster(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        buffered_request_timeout: Duration::from_secs(120),
        max_buffered_requests: 1,
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

    cluster
        .buffered_requests
        .until(assert_count(1))
        .await
        .context("balancer should buffer the request before any agent joins")?;

    let mut agent_child = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        management_addr: cluster.addresses.management,
        name: Some("buffered-agent".to_owned()),
        slots: 4,
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

    agent_child.start_kill()?;
    agent_child.wait().await?;

    cluster.shutdown().await?;

    Ok(())
}
