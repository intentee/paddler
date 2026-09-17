#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const CONTEXT_SIZE: u32 = 256;
const BATCH_SIZE: usize = 128;
const SLOT_COUNT: i32 = 2;

fn long_prompt(subject: &str) -> String {
    format!("{subject} ").repeat(200)
}

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_restages_the_surviving_prompt_after_an_eviction() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: SLOT_COUNT,
        }],
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                n_batch: BATCH_SIZE,
                context_size: CONTEXT_SIZE,
                temperature: 0.0,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;

    let collected_results =
        futures_util::future::join_all(["alpha", "beta"].into_iter().map(|subject| {
            cluster.continue_from_raw_prompt(
                CancellationToken::new(),
                &ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 4,
                    raw_prompt: long_prompt(subject),
                },
            )
        }))
        .await;

    let summaries: Vec<_> = collected_results
        .iter()
        .filter_map(|collected| collected.as_ref().ok())
        .flat_map(|collected| collected.token_results.iter())
        .filter_map(|result| match &result.token_result {
            GeneratedTokenResult::Done(summary) => Some(summary),
            _ => None,
        })
        .collect();

    assert!(
        !summaries.is_empty(),
        "at least one prompt must survive the KV cache pressure and finish"
    );

    for summary in summaries {
        assert!(
            summary.usage.prompt_tokens <= u64::from(CONTEXT_SIZE),
            "a prompt re-staged after a rollback must be counted once, got {} prompt tokens",
            summary.usage.prompt_tokens
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
