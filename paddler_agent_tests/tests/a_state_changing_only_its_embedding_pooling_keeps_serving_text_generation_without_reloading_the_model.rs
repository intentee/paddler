#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_tests::swappable_model_file::SwappableModelFile;
use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn a_state_changing_only_its_embedding_pooling_keeps_serving_text_generation_without_reloading_the_model()
 {
    let swappable_model_file = SwappableModelFile::link_to(
        &qwen3_0_6b()
            .cached_path()
            .expect("the Qwen3 model must be in the Hugging Face cache"),
    )
    .expect("the model must be linked into a swappable model file");
    let serving_last_token_embeddings = BalancerDesiredState {
        embeddings: EmbeddingParameters {
            pooling_type: PoolingType::Last,
            ..EmbeddingParameters::default()
        },
        model: swappable_model_file
            .agent_desired_model()
            .expect("the swappable model file must be at an absolute path"),
        ..qwen3_desired_state()
    };
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(serving_last_token_embeddings.clone())
        .await
        .expect("the desired state must be served");
    swappable_model_file
        .replace_with_an_unloadable_file()
        .expect("the unloadable file must replace the model");
    serving_pipeline_arbiter
        .apply(BalancerDesiredState {
            embeddings: EmbeddingParameters {
                pooling_type: PoolingType::Mean,
                ..serving_last_token_embeddings.embeddings.clone()
            },
            ..serving_last_token_embeddings
        })
        .await;
    serving_pipeline_arbiter.conclude_next_startup().await;

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert_eq!(
        status.state_application_status,
        AgentStateApplicationStatus::Applied
    );
    assert!(matches!(
        status.runtime,
        AgentRuntimeStatus::Serving { slots_total: 1, .. }
    ));

    serving_pipeline_arbiter.shut_down().await;
}
