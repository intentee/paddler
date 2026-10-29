#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::uncompilable_chat_template::uncompilable_chat_template;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_startup_releases_its_model_when_the_agent_shuts_down() {
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(1);
    let model_weights_residency = serving_pipeline_arbiter
        .inference_runtime_context
        .vram_residencies
        .model_weights
        .clone();

    serving_pipeline_arbiter
        .serve(desired_state_with_chat_template_override(
            BalancerDesiredState {
                decision: DecisionSettings::default(),
                ..KevCheckpoint::Kev0_8b
                    .desired_state()
                    .expect("the kev desired state must name absolute paths")
            },
            uncompilable_chat_template(),
        ))
        .await;
    assert!(model_weights_residency.try_occupy().is_err());

    serving_pipeline_arbiter.shut_down().await;

    assert!(model_weights_residency.try_occupy().is_ok());
}
