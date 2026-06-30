#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use log::LevelFilter;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn agent_evicts_largest_sequence_under_kv_cache_pressure() -> Result<()> {
    log::set_max_level(LevelFilter::Trace);

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

    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: vec![AgentConfig {
                name: "test-agent".to_owned(),
                slot_count: 1,
            }],
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters,
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
        },
    )
    .await?;

    let collected = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt_collected(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 4096,
            raw_prompt: "Write an exhaustive, never-ending encyclopedia entry that lists every fact about the natural world in extreme detail:".to_owned(),
        })
        .await?;

    let evicted = collected.token_results.iter().any(|result| {
        matches!(
            &result.token_result,
            GeneratedTokenResult::SequenceEvictedUnderKvCachePressure
        )
    });

    assert!(
        evicted,
        "the sole sequence must be evicted once its KV cache footprint exceeds the context size"
    );

    cluster.shutdown().await?;

    Ok(())
}
