#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::agents_status::AgentsStatus;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::in_process_cluster::InProcessCluster;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn management_two_agents_stream_subscribers_receive_slot_usage_updates() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
        model: AgentDesiredModel::HuggingFace(reference),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let mut cluster = InProcessCluster::start(InProcessClusterParams {
        spawn_agent: true,
        slots_per_agent: 1,
        desired_state,
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have registered one agent")?
        .clone();

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let token_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 8,
            raw_prompt: "Count to three".to_owned(),
        })
        .await?;

    cluster
        .agents
        .until(AgentsStatus::slots_processing_is(&agent_id, 1))
        .await
        .context("agents_stream must emit a snapshot showing slot usage")?;

    collect_generated_tokens(token_stream).await?;

    cluster
        .agents
        .until(AgentsStatus::slots_processing_is(&agent_id, 0))
        .await
        .context("agents_stream must emit a snapshot showing the slot was released")?;

    cluster.shutdown().await?;

    Ok(())
}
