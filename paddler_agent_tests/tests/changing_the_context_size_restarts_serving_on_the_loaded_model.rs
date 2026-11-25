#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_tests::swappable_model_file::SwappableModelFile;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::constants::SERVED_CONTEXT_SIZE;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;

#[tokio::test(flavor = "multi_thread")]
async fn changing_the_context_size_restarts_serving_on_the_loaded_model() {
    let swappable_model_file = SwappableModelFile::link_to(
        &qwen3_5_0_8b()
            .cached_path()
            .expect("the Qwen3.5 model must be in the Hugging Face cache"),
    )
    .expect("the model must be linked into a swappable model file");
    let initial_desired_state = BalancerDesiredState {
        model: swappable_model_file
            .agent_desired_model()
            .expect("the swappable model file must be at an absolute path"),
        ..qwen3_5_0_8b().into_desired_state()
    };
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(initial_desired_state.clone())
        .await
        .expect("the desired state must be served");

    swappable_model_file
        .replace_with_an_unloadable_file()
        .expect("the unloadable file must replace the model");

    serving_pipeline_arbiter
        .serve(BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                context_size: ModelDefaultOr::Explicit(
                    NonZeroU32::new(SERVED_CONTEXT_SIZE.get() / 2)
                        .expect("half of the served context must hold tokens"),
                ),
                ..initial_desired_state.model_runtime_parameters.clone()
            },
            ..initial_desired_state
        })
        .await
        .expect("the desired state must be served");
    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .get_state_application_status(),
        AgentStateApplicationStatus::Applied
    );

    serving_pipeline_arbiter.shut_down().await;
}
