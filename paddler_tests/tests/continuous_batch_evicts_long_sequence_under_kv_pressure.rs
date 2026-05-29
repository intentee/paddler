#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::inference_parameters::InferenceParameters;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;
use paddler_tests::token_result_with_producer::TokenResultWithProducer;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_evicts_long_sequence_under_kv_pressure() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut inference_parameters = InferenceParameters {
        n_gpu_layers: gpu_layer_count,
        ..InferenceParameters::default()
    };

    inference_parameters.n_batch = 256;
    inference_parameters.context_size = 256;
    inference_parameters.temperature = 0.0;

    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 2,
        }],
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;

    let long_prompt = "Describe in great detail how the process of photosynthesis works in plants. Cover the light-dependent reactions, the Calvin cycle, the role of chlorophyll, the thylakoid membrane, and the stroma. Explain how water and carbon dioxide are converted to glucose and oxygen. Discuss the evolutionary history of this process and its importance throughout the biosphere, and then give a long essay response.";

    let long_params = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 200,
        raw_prompt: long_prompt.to_owned(),
    };
    let short_params = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 20,
        raw_prompt: "Hi".to_owned(),
    };
    let (long_collected, short_collected) = tokio::join!(
        cluster.continue_from_raw_prompt(&long_params),
        cluster.continue_from_raw_prompt(&short_params),
    );

    let long_collected = long_collected?;
    let short_collected = short_collected?;

    let long_was_evicted = long_collected.token_results.iter().any(|result| {
        matches!(&result.token_result, GeneratedTokenResult::SamplerError(message) if message.contains("evicted"))
    });

    assert!(
        long_was_evicted,
        "long prompt must be evicted under KV pressure"
    );
    assert!(matches!(
        short_collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    cluster.shutdown().await?;

    Ok(())
}
