#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use paddler_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::model_download_status::ModelDownloadStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::decision_desired_state::DecisionDesiredState;
use paddler_tests::serving_pipeline_arbiter::ServingPipelineArbiter;

const SLOTS_TOO_FEW_FOR_DECISIONS: u16 = 1;

#[tokio::test(flavor = "multi_thread")]
async fn a_kev_model_with_too_few_slots_serves_text_generation_without_decisions() {
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(SLOTS_TOO_FEW_FOR_DECISIONS)
        .expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(DecisionDesiredState::default().into_balancer_desired_state())
        .await
        .expect("the desired state must be served");

    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status,
        AgentStatus {
            desired_slots_total: SLOTS_TOO_FEW_FOR_DECISIONS,
            download_status: ModelDownloadStatus::NotDownloading,
            issues: BTreeSet::from([AgentIssue::SlotsInsufficientForDecisions(
                SlotsInsufficientForDecisionsParams {
                    desired_slots: SLOTS_TOO_FEW_FOR_DECISIONS,
                    required_slots: DECISION_SLOTS_MINIMUM.get(),
                }
            )]),
            model_path: Some(
                kev_0_8b()
                    .cached_path()
                    .expect("the kev model must be in the Hugging Face cache")
                    .display()
                    .to_string()
            ),
            runtime: AgentRuntimeStatus::Serving {
                decisions: AgentDecisionsServing::DecisionModelNotServed,
                slots_total: u64::from(SLOTS_TOO_FEW_FOR_DECISIONS),
            },
            state_application_status: AgentStateApplicationStatus::Applied,
            uses_chat_template_override: false,
        }
    );

    serving_pipeline_arbiter.shut_down().await;
}
