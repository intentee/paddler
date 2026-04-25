#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::in_process_cluster::InProcessCluster;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

const PARTIAL_GPU_LAYER_COUNT: u32 = 14;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_generates_tokens_with_partial_layer_offload() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard { reference, .. } = qwen3_0_6b();

    let inference_parameters = device.inference_parameters_for_full_offload(PARTIAL_GPU_LAYER_COUNT);

    let cluster = InProcessCluster::start(InProcessClusterParams {
        agent_count: 1,
        slots_per_agent: 1,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        },
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 16,
            raw_prompt: "Count from 1 to 5:".to_owned(),
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count > 0);
    assert!(matches!(
        collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    cluster.shutdown().await?;

    Ok(())
}
