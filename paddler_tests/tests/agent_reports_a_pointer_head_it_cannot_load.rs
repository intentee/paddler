#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_issue::AgentIssue;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_a_pointer_head_it_cannot_load() {
    let mut cluster = start_decision_cluster(DecisionClusterParams {
        pointer_head_fixture: "invalid.gguf",
        wait_for_slots_ready: false,
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(
                issue,
                AgentIssue::PointerHeadCannotBeLoaded(pointer_head_path)
                    if pointer_head_path.model_path.ends_with("/fixtures/invalid.gguf")
            )
        })
        .await
        .expect("the agent must report the unreadable pointer head");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
