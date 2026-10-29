#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_tests::kev_checkpoint::KevCheckpoint;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn a_replacement_superseded_by_its_own_state_keeps_reporting_the_model_metadata() {
    let serving_decisions = KevCheckpoint::Kev0_8b
        .desired_state()
        .expect("the kev desired state must name absolute paths");
    let serving_without_decisions = BalancerDesiredState {
        decision: DecisionSettings::default(),
        ..serving_decisions.clone()
    };
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get());

    serving_pipeline_arbiter.serve(serving_decisions).await;

    let served_model_metadata = serving_pipeline_arbiter
        .inference_runtime_context
        .model_metadata_holder
        .get_model_metadata();

    serving_pipeline_arbiter
        .apply(serving_without_decisions.clone())
        .await;
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter
        .apply(serving_without_decisions)
        .await;
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter.conclude_next_progress().await;

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
