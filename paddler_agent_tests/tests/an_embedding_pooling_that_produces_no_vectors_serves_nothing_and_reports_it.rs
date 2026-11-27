#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

#[tokio::test(flavor = "multi_thread")]
async fn an_embedding_pooling_that_produces_no_vectors_serves_nothing_and_reports_it() {
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .apply(
            nomic_embed_text_v1_5().into_desired_state_with_embedding_parameters(
                EmbeddingParameters {
                    pooling_type: PoolingType::Rank,
                    ..EmbeddingParameters::default()
                },
            ),
        )
        .await;
    serving_pipeline_arbiter.conclude_next_startup().await;

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert!(
        status
            .issues
            .iter()
            .any(|issue| matches!(issue, AgentIssue::EmbeddingPoolingProducesNoVectors(_)))
    );
    assert_eq!(status.runtime, AgentRuntimeStatus::Idle);

    serving_pipeline_arbiter.shut_down().await;
}
