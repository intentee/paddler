#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;

const SLOTS_TOO_FEW_FOR_DECISIONS: u16 = 1;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_model_with_too_few_slots_serves_nothing_and_reports_its_slots() {
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(SLOTS_TOO_FEW_FOR_DECISIONS)
        .expect("the model downloader must start");

    serving_pipeline_arbiter
        .apply(DecisionDesiredState::default().into_balancer_desired_state())
        .await;
    serving_pipeline_arbiter.conclude_next_startup().await;

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert!(
        status
            .issues
            .contains(&AgentIssue::SlotsInsufficientForDecisions(
                SlotsInsufficientForDecisionsParams {
                    desired_slots: SLOTS_TOO_FEW_FOR_DECISIONS,
                    required_slots: DECISION_SLOTS_MINIMUM.get(),
                }
            ))
    );
    assert_eq!(status.runtime, AgentRuntimeStatus::Idle);
    assert_eq!(
        status.state_application_status,
        AgentStateApplicationStatus::AttemptedAndRetrying
    );

    serving_pipeline_arbiter.shut_down().await;
}
