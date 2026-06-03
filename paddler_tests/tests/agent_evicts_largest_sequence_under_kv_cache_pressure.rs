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

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_evicts_largest_sequence_under_kv_cache_pressure() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let inference_parameters = InferenceParameters {
        n_gpu_layers: gpu_layer_count,
        n_batch: 256,
        context_size: 256,
        temperature: 0.0,
        ..InferenceParameters::default()
    };

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
            max_tokens: 4096,
            raw_prompt: "Write an exhaustive, never-ending encyclopedia entry that lists every fact about the natural world in extreme detail:".to_owned(),
        })
        .await?;

    let evicted = collected.token_results.iter().any(|result| {
        matches!(
            &result.token_result,
            GeneratedTokenResult::SamplerError(message) if message.contains("evicted")
        )
    });

    assert!(
        evicted,
        "the sole sequence must be evicted once its KV cache footprint exceeds the context size"
    );

    cluster.shutdown().await?;

    Ok(())
}
