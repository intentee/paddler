#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::kev_checkpoint::KevCheckpoint;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;
use crate::swappable_model_file::SwappableModelFile;

#[tokio::test(flavor = "multi_thread")]
async fn enabling_decisions_reuses_the_loaded_model() {
    let swappable_model_file =
        SwappableModelFile::link_to(&KevCheckpoint::Kev0_8b.target_path("model.gguf"));
    let decisions_desired_state = BalancerDesiredState {
        model: swappable_model_file.agent_desired_model(),
        ..KevCheckpoint::Kev0_8b
            .decision_desired_state()
            .into_balancer_desired_state()
            .expect("the kev desired state must name absolute paths")
    };
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get());

    serving_pipeline_arbiter
        .serve(BalancerDesiredState {
            decision: DecisionSettings::default(),
            ..decisions_desired_state.clone()
        })
        .await;

    swappable_model_file.replace_with_an_unloadable_file();

    serving_pipeline_arbiter
        .serve(decisions_desired_state)
        .await;
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
