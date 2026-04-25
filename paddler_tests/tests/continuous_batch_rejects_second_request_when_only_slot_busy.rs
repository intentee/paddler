#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
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

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_rejects_second_request_when_only_slot_busy() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = InProcessCluster::start(InProcessClusterParams {
        agent_count: 1,
        slots_per_agent: 1,
        max_buffered_requests: 0,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        },
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut first_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 100,
            raw_prompt: "Tell me a long story about an explorer".to_owned(),
        })
        .await?;

    let _first_message = first_stream
        .next()
        .await
        .context("first stream must yield at least one message")?;

    cluster
        .agents
        .until(AgentsStatus::slots_processing_is(&agent_id, 1))
        .await
        .context("first request should occupy the only slot")?;

    let second_outcome = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await;

    let second_failed = match second_outcome {
        Err(_) => true,
        Ok(stream) => {
            let collected = collect_generated_tokens(stream).await;

            collected.is_err()
        }
    };

    assert!(
        second_failed,
        "second request must be rejected when the only slot is busy and buffering is disabled"
    );

    drop(first_stream);

    cluster.shutdown().await?;

    Ok(())
}
