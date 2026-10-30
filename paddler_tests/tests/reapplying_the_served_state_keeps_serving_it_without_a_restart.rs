#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::desired_state_reconciled_discriminant::desired_state_reconciled_discriminant;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn reapplying_the_served_state_keeps_serving_it_without_a_restart() {
    let desired_state = qwen3_0_6b().into_desired_state();
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(1);

    serving_pipeline_arbiter.serve(desired_state.clone()).await;

    let status_while_serving = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert_eq!(
        discriminant(&serving_pipeline_arbiter.apply(desired_state).await),
        desired_state_reconciled_discriminant()
    );

    serving_pipeline_arbiter.conclude_next_progress().await;

    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status,
        status_while_serving
    );

    serving_pipeline_arbiter.shut_down().await;
}
