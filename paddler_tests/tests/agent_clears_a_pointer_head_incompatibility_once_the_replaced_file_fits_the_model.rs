#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use crate::replaceable_pointer_head::ReplaceablePointerHead;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
use paddler_messaging::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_clears_a_pointer_head_incompatibility_once_the_replaced_file_fits_the_model() {
    let pointer_head = ReplaceablePointerHead::holding_fixture(
        "qwen3_5_0_8b_synthetic_pointer_head_with_wrong_hidden_size.gguf",
    );
    let hidden_size_mismatch =
        AgentIssue::PointerHeadIncompatibleWithModel(PointerHeadIncompatibleWithModelParams {
            incompatibility: PointerHeadIncompatibility::HiddenSizeMismatch {
                model_hidden_size: 1024,
                pointer_head_hidden_size: 512,
            },
            pointer_head_path: ModelPath::from(pointer_head.path.as_path()),
        });
    let mut cluster = start_decision_cluster(DecisionClusterParams {
        pointer_head: pointer_head.path.clone(),
        wait_for_slots_ready: false,
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| *issue == hidden_size_mismatch)
        .await
        .expect("the agent must report the incompatible pointer head");

    pointer_head.replace_with_fixture(QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE);

    let serving_snapshot = cluster
        .wait_for_first_agent_to_serve(AgentDecisionsServing::Served)
        .await
        .expect("the agent must serve decisions with the replaced pointer head");

    assert_eq!(
        serving_snapshot
            .agents
            .into_iter()
            .map(|agent| agent.status.issues)
            .collect::<Vec<_>>(),
        vec![BTreeSet::new()]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
