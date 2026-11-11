#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_tests::swappable_model_file::SwappableModelFile;

#[tokio::test(flavor = "multi_thread")]
async fn enabling_decisions_reuses_the_loaded_model() {
    let swappable_model_file =
        SwappableModelFile::link_to(&KevCheckpoint::Kev0_8b.target_path("model.gguf"))
            .expect("the model must be linked into a swappable model file");
    let decisions_desired_state = BalancerDesiredState {
        model: swappable_model_file
            .agent_desired_model()
            .expect("the swappable model file must be at an absolute path"),
        ..KevCheckpoint::Kev0_8b
            .decision_desired_state()
            .into_balancer_desired_state()
            .expect("the kev desired state must name absolute paths")
    };
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get())
        .expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(BalancerDesiredState {
            decision: DecisionSettings::default(),
            ..decisions_desired_state.clone()
        })
        .await
        .expect("the desired state must be served");

    swappable_model_file
        .replace_with_an_unloadable_file()
        .expect("the unloadable file must replace the model");

    serving_pipeline_arbiter
        .serve(decisions_desired_state)
        .await
        .expect("the desired state must be served");
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
