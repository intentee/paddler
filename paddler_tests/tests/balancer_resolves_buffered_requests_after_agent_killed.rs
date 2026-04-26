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
use paddler_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_client::Message;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_resolves_buffered_requests_after_agent_killed() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 1,
        slots_per_agent: 2,
        wait_for_slots_ready: true,
        buffered_request_timeout: Duration::from_secs(120),
        max_buffered_requests: 10,
        agent_name_prefix: "removal-agent-primary".to_owned(),
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

    let mut secondary_agent = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        management_addr: cluster.addresses.management,
        name: Some("removal-agent-secondary".to_owned()),
        slots: 2,
    })?;

    cluster
        .agents
        .until(|snapshot| snapshot.agents.len() == 2)
        .await
        .context("both agents should register")?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut streams = Vec::new();

    for _ in 0..3 {
        let stream = inference_client
            .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 10,
                raw_prompt: "Hello".to_owned(),
            })
            .await?;

        streams.push(stream);
    }

    secondary_agent.start_kill()?;
    secondary_agent.wait().await?;

    let mut total_resolved = 0;

    for mut stream in streams {
        if let Some(item) = stream.next().await {
            match item {
                Ok(Message::Response(_) | Message::Error(_)) | Err(_) => total_resolved += 1,
            }
        }
    }

    assert_eq!(
        total_resolved, 3,
        "all 3 buffered requests must resolve after one agent is killed"
    );

    cluster.shutdown().await?;

    Ok(())
}
