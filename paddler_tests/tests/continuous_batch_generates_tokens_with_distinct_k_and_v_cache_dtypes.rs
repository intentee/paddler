#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::inference_parameters::InferenceParameters;
use paddler::kv_cache_dtype::KvCacheDtype;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;
use paddler_tests::token_result_with_producer::TokenResultWithProducer;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_generates_tokens_with_distinct_k_and_v_cache_dtypes() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut inference_parameters = InferenceParameters {
        n_gpu_layers: gpu_layer_count,
        ..InferenceParameters::default()
    };

    inference_parameters.k_cache_dtype = KvCacheDtype::Q8_0;
    inference_parameters.v_cache_dtype = KvCacheDtype::F16;

    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
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

    let collected = cluster
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 8,
            raw_prompt: "Count from 1 to 3:".to_owned(),
        })
        .await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(token_count > 0);
    assert!(matches!(
        collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    cluster.shutdown().await?;

    Ok(())
}
