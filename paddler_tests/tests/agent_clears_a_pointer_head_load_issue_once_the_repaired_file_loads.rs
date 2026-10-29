#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use crate::replaceable_pointer_head::ReplaceablePointerHead;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_desired_state::DecisionDesiredState;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_clears_a_pointer_head_load_issue_once_the_repaired_file_loads() {
    let pointer_head = ReplaceablePointerHead::holding_fixture("invalid.gguf");
    let unloadable_pointer_head =
        AgentIssue::PointerHeadCannotBeLoaded(ModelPath::from(pointer_head.path.as_path()));
    let mut cluster = start_decision_cluster(DecisionClusterParams {
        desired_state: DecisionDesiredState {
            pointer_head: pointer_head.path.clone(),
            ..DecisionDesiredState::default()
        },
        wait_for_slots_ready: false,
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| *issue == unloadable_pointer_head)
        .await
        .expect("the agent must report the unreadable pointer head");

    pointer_head.replace_with_fixture(QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE);

    let serving_snapshot = cluster
        .wait_for_first_agent_to_serve(AgentDecisionsServing::Served)
        .await
        .expect("the agent must serve decisions with the repaired pointer head");

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
