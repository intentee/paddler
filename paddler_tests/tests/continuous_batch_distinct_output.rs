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
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn two_concurrent_prompts_produce_distinct_outputs() -> Result<()> {
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

    let cluster = InProcessCluster::start(InProcessClusterParams {
        spawn_agent: true,
        slots_per_agent: 2,
        desired_state,
        wait_for_slots_ready: true,
        ..InProcessClusterParams::default()
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream_a = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: "Count from one to ten in English: one, two,".to_owned(),
        })
        .await?;

    let stream_b = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: "The capital of France is".to_owned(),
        })
        .await?;

    let (collected_a, collected_b) = tokio::join!(
        collect_generated_tokens(stream_a),
        collect_generated_tokens(stream_b),
    );

    let collected_a = collected_a?;
    let collected_b = collected_b?;

    assert!(
        !collected_a.text.is_empty(),
        "first concurrent prompt should produce tokens"
    );
    assert!(
        !collected_b.text.is_empty(),
        "second concurrent prompt should produce tokens"
    );
    assert_ne!(
        collected_a.text, collected_b.text,
        "two different prompts should produce different outputs"
    );

    cluster.shutdown().await?;

    Ok(())
}
