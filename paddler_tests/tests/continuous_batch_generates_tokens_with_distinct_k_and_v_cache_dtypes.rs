#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_client::token_result_with_producer::TokenResultWithProducer;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::kv_cache_dtype::KvCacheDtype;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

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

    inference_parameters.k_cache_dtype = KvCacheDtype::Q80;
    inference_parameters.v_cache_dtype = KvCacheDtype::F16;

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
