#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings_types::decisions_capability::DecisionsCapability;
use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_tests::swappable_model_file::SwappableModelFile;
use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_inference_parameters::desired_sampling_parameters::DesiredSamplingParameters;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

#[tokio::test(flavor = "multi_thread")]
async fn a_state_changing_only_its_sampling_keeps_serving_decisions_without_reloading_the_model() {
    let swappable_model_file = SwappableModelFile::link_to(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
    )
    .expect("the model must be linked into a swappable model file");
    let serving_decisions = BalancerDesiredState {
        model: swappable_model_file
            .agent_desired_model()
            .expect("the swappable model file must be at an absolute path"),
        ..DecisionDesiredState::default().into_balancer_desired_state()
    };
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(DECISION_SLOTS_WITH_A_QUESTION_LANE.get())
            .expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(serving_decisions.clone())
        .await
        .expect("the desired state must be served");
    swappable_model_file
        .replace_with_an_unloadable_file()
        .expect("the unloadable file must replace the model");
    serving_pipeline_arbiter
        .apply(BalancerDesiredState {
            text_generation: BalancerTextGenerationSettings {
                sampling_parameters: DesiredSamplingParameters::default(),
                ..serving_decisions.text_generation.clone()
            },
            ..serving_decisions
        })
        .await;
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
        AgentRuntimeStatus::Serving {
            capabilities: ModelCapabilities {
                decisions: DecisionsCapability::Supported(_),
                ..
            },
            slots_total,
        } if slots_total == u64::from(DECISION_SLOTS_WITH_A_QUESTION_LANE.get())
    ));

    serving_pipeline_arbiter.shut_down().await;
}
