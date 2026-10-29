#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
use paddler_messaging::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;
use paddler_test_cluster_harness::fixture_path::fixture_path;
use paddler_test_cluster_harness::model_card::lfm2_colbert_350m::lfm2_colbert_350m;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_a_pointer_head_wider_than_the_hidden_states_the_model_outputs() {
    let hidden_size_mismatch =
        AgentIssue::PointerHeadIncompatibleWithModel(PointerHeadIncompatibleWithModelParams {
            incompatibility: PointerHeadIncompatibility::HiddenSizeMismatch {
                model_hidden_size: 128,
                pointer_head_hidden_size: 1024,
            },
            pointer_head_path: ModelPath::from(
                fixture_path(QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE).as_path(),
            ),
        });
    let mut cluster = start_decision_cluster(DecisionClusterParams {
        model_card: lfm2_colbert_350m(),
        wait_for_slots_ready: false,
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| *issue == hidden_size_mismatch)
        .await
        .expect("the agent must report the pointer head as wider than the hidden states it reads");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
