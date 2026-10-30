#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_tests::kev_checkpoint::KevCheckpoint;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;
use crate::swappable_model_file::SwappableModelFile;

#[tokio::test(flavor = "multi_thread")]
async fn changing_the_context_size_restarts_serving_on_the_loaded_model() {
    let swappable_model_file =
        SwappableModelFile::link_to(&KevCheckpoint::Kev0_8b.target_path("model.gguf"));
    let initial_desired_state = BalancerDesiredState {
        decision: DecisionSettings::default(),
        model: swappable_model_file.agent_desired_model(),
        ..KevCheckpoint::Kev0_8b
            .decision_desired_state()
            .into_balancer_desired_state()
            .expect("the kev desired state must name absolute paths")
    };
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(1);

    serving_pipeline_arbiter
        .serve(initial_desired_state.clone())
        .await;

    swappable_model_file.replace_with_an_unloadable_file();

    serving_pipeline_arbiter
        .serve(BalancerDesiredState {
            model_runtime_parameters: ModelRuntimeParameters {
                context_size: NonZeroU32::new(
                    initial_desired_state
                        .model_runtime_parameters
                        .context_size
                        .get()
                        / 2,
                )
                .expect("half of the initial context must hold tokens"),
                ..initial_desired_state.model_runtime_parameters.clone()
            },
            ..initial_desired_state
        })
        .await;
    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .get_state_application_status(),
        AgentStateApplicationStatus::Applied
    );

    serving_pipeline_arbiter.shut_down().await;
}
