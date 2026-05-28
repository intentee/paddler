#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_in_process_cluster::start_in_process_cluster;
use paddler_tests::token_result_with_producer::TokenResultWithProducer;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_smoke_generates_tokens() -> Result<()> {
    let device = current_test_device()?;

    device
        .require_available()
        .context("selected device is unavailable")?;

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

    let cluster = start_in_process_cluster(InProcessClusterParams {
        agent: Some(AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
        }),
        desired_state,
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await
    .context("failed to start in-process cluster with Qwen3 0.6B")?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 16,
            raw_prompt: "Count from 1 to 5:".to_owned(),
        })
        .await
        .context("failed to POST /api/v1/continue_from_raw_prompt")?;

    let collected = collect_generated_tokens(stream).await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(
        token_count > 0,
        "smoke test on {} produced no tokens",
        device.name()
    );

    assert!(
        matches!(
            collected.token_results.last(),
            Some(TokenResultWithProducer {
                token_result: GeneratedTokenResult::Done(_),
                ..
            })
        ),
        "smoke test stream did not terminate with Done"
    );

    cluster.shutdown().await?;

    Ok(())
}
