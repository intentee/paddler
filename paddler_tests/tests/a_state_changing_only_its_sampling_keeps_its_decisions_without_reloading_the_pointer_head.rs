#![cfg(feature = "tests_that_use_llms")]

use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_tests::swappable_model_file::SwappableModelFile;

#[tokio::test(flavor = "multi_thread")]
async fn a_state_changing_only_its_sampling_keeps_its_decisions_without_reloading_the_pointer_head()
{
    let swappable_pointer_head =
        SwappableModelFile::link_to(&KevCheckpoint::Kev0_8b.target_path("pointer_head.gguf"))
            .expect("the model must be linked into a swappable model file");
    let serving_decisions = BalancerDesiredState {
        decision: DecisionSettings {
            pointer_head: swappable_pointer_head
                .agent_desired_model()
                .expect("the swappable model file must be at an absolute path"),
        },
        ..KevCheckpoint::Kev0_8b
            .decision_desired_state()
            .into_balancer_desired_state()
            .expect("the kev desired state must name absolute paths")
    };
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get())
        .expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(serving_decisions.clone())
        .await
        .expect("the desired state must be served");
    swappable_pointer_head
        .replace_with_an_unloadable_file()
        .expect("the unloadable file must replace the model");
    serving_pipeline_arbiter
        .apply(BalancerDesiredState {
            text_generation: BalancerTextGenerationSettings {
                sampling_parameters: SamplingParameters::deterministic(),
                ..serving_decisions.text_generation.clone()
            },
            ..serving_decisions
        })
        .await;
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter.conclude_next_progress().await;

    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status
            .state_application_status,
        AgentStateApplicationStatus::Applied
    );
    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status
            .runtime,
        AgentRuntimeStatus::Serving {
            decisions: AgentDecisionsServing::Served,
            slots_total: u64::from(DECISION_SLOTS_MINIMUM.get()),
        }
    );

    serving_pipeline_arbiter.shut_down().await;
}
