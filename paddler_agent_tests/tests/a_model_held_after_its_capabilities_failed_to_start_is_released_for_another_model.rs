#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_tests::swappable_model_file::SwappableModelFile;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template::uncompilable_chat_template;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_held_after_its_capabilities_failed_to_start_is_released_for_another_model() {
    let another_model_file = SwappableModelFile::link_to(
        &qwen3_0_6b()
            .cached_path()
            .expect("the Qwen3 model must be in the Hugging Face cache"),
    )
    .expect("the model must be linked at another path");
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(desired_state_with_chat_template_override(
            qwen3_desired_state(),
            uncompilable_chat_template(),
        ))
        .await
        .expect("the desired state with an uncompilable chat template must be reconciled");
    serving_pipeline_arbiter
        .serve(BalancerDesiredState {
            model: another_model_file
                .agent_desired_model()
                .expect("the linked model file must be at an absolute path"),
            ..qwen3_desired_state()
        })
        .await
        .expect("the desired state of another model must supersede the retried startup");
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
