#![cfg(feature = "tests_that_use_llms")]

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_tests::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn a_replacement_superseded_by_its_own_state_keeps_reporting_the_model_metadata() {
    let served_state = qwen3_5_0_8b().into_desired_state();
    let replacing_state = BalancerDesiredState {
        embeddings: EmbeddingParameters {
            pooling_type: PoolingType::Mean,
            ..served_state.embeddings.clone()
        },
        ..served_state.clone()
    };
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(served_state)
        .await
        .expect("the desired state must be served");

    let served_model_metadata = serving_pipeline_arbiter
        .inference_runtime_context
        .model_metadata_holder
        .get_model_metadata();

    serving_pipeline_arbiter
        .apply(replacing_state.clone())
        .await;
    serving_pipeline_arbiter.apply(replacing_state).await;
    serving_pipeline_arbiter.conclude_next_startup().await;
    serving_pipeline_arbiter.conclude_next_startup().await;

    assert_ne!(served_model_metadata, None);
    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .model_metadata_holder
            .get_model_metadata(),
        served_model_metadata
    );

    serving_pipeline_arbiter.shut_down().await;
}
