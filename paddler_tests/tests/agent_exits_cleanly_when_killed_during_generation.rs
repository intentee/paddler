#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::agents_status::AgentsStatus;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_tests::terminate_subprocess::terminate_subprocess;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn agent_exits_cleanly_when_killed_during_generation() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
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

    let agent_child = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        management_addr: cluster.addresses.management,
        name: Some("graceful-shutdown-agent".to_owned()),
        slots: 2,
    })?;

    let snapshot = cluster
        .agents
        .until(|snapshot| {
            snapshot.agents.len() == 1
                && snapshot.agents.iter().any(|agent| agent.slots_total >= 2)
        })
        .await
        .context("agent must register with slots before generation starts")?;

    let agent_id = snapshot
        .agents
        .first()
        .context("snapshot must contain registered agent")?
        .id
        .clone();

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 1000,
            raw_prompt: "Write a long story".to_owned(),
        })
        .await?;

    let _first_message = stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("stream must yield at least one message"))?;

    cluster
        .agents
        .until(AgentsStatus::slots_processing_is(&agent_id, 1))
        .await?;

    let exit_status = terminate_subprocess(agent_child).await?;

    cluster
        .agents
        .until(AgentsStatus::agent_count_is(0))
        .await?;

    drop(stream);

    cluster.shutdown().await?;

    assert!(
        exit_status.code().is_some() || exit_status.success(),
        "agent must exit cleanly (no abnormal termination); got {exit_status:?}"
    );

    Ok(())
}
