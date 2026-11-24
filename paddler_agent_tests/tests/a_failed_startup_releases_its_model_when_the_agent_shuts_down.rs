#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template::uncompilable_chat_template;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_startup_releases_its_model_when_the_agent_shuts_down() {
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");
    let model_weights_residency = serving_pipeline_arbiter
        .inference_runtime_context
        .vram_residencies
        .model_weights
        .clone();

    serving_pipeline_arbiter
        .serve(desired_state_with_chat_template_override(
            qwen3_5_0_8b().into_desired_state(),
            uncompilable_chat_template(),
        ))
        .await
        .expect("the desired state must be served");
    assert!(model_weights_residency.try_occupy().is_err());

    serving_pipeline_arbiter.shut_down().await;

    assert!(model_weights_residency.try_occupy().is_ok());
}
