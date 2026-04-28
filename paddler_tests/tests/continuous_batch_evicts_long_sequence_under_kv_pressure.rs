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

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_evicts_long_sequence_under_kv_pressure() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut inference_parameters = device.inference_parameters_for_full_offload(gpu_layer_count);

    inference_parameters.batch_n_tokens = 256;
    inference_parameters.context_size = 256;
    inference_parameters.temperature = 0.0;

    let cluster = InProcessCluster::start(InProcessClusterParams {
        spawn_agent: true,
        slots_per_agent: 2,
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

    let long_prompt = "Describe in great detail how the process of photosynthesis works in plants. Cover the light-dependent reactions, the Calvin cycle, the role of chlorophyll, the thylakoid membrane, and the stroma. Explain how water and carbon dioxide are converted to glucose and oxygen. Discuss the evolutionary history of this process and its importance throughout the biosphere, and then give a long essay response.";

    let long_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 200,
            raw_prompt: long_prompt.to_owned(),
        })
        .await?;

    let short_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: "Hi".to_owned(),
        })
        .await?;

    let (long_collected, short_collected) = tokio::join!(
        collect_generated_tokens(long_stream),
        collect_generated_tokens(short_stream),
    );

    let long_collected = long_collected?;
    let short_collected = short_collected?;

    let long_was_evicted = long_collected.token_results.iter().any(|result| {
        matches!(result, GeneratedTokenResult::SamplerError(message) if message.contains("evicted"))
    });

    assert!(
        long_was_evicted,
        "long prompt must be evicted under KV pressure"
    );
    assert!(matches!(
        short_collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    cluster.shutdown().await?;

    Ok(())
}
