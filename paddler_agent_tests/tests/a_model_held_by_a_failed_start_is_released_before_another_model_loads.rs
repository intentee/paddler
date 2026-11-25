#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use paddler_agent_pipeline_harness::desired_state_reconciled_discriminant::desired_state_reconciled_discriminant;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template::uncompilable_chat_template;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_held_by_a_failed_start_is_released_before_another_model_loads() {
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(desired_state_with_chat_template_override(
                    qwen3_5_0_8b().into_desired_state(),
                    uncompilable_chat_template(),
                ))
                .await
        ),
        desired_state_reconciled_discriminant()
    );

    serving_pipeline_arbiter.conclude_next_startup().await;

    assert_eq!(
        discriminant(&serving_pipeline_arbiter.apply(qwen3_desired_state()).await),
        desired_state_reconciled_discriminant()
    );

    serving_pipeline_arbiter.conclude_next_startup().await;
    serving_pipeline_arbiter.conclude_next_startup().await;

    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status
            .runtime,
        AgentRuntimeStatus::Serving {
            decisions: AgentDecisionsServing::NotServed,
            slots_total: 1,
        }
    );

    serving_pipeline_arbiter.shut_down().await;
}
