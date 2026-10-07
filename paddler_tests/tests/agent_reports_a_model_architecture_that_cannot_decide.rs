#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_architecture_unsupported_for_decisions_params::ModelArchitectureUnsupportedForDecisionsParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_a_model_architecture_that_cannot_decide() {
    let mut cluster = start_decision_cluster(DecisionClusterParams {
        model_card: qwen3_0_6b(),
        wait_for_slots_ready: false,
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(
                issue,
                AgentIssue::ModelArchitectureUnsupportedForDecisions(
                    ModelArchitectureUnsupportedForDecisionsParams { architecture, .. }
                ) if architecture == "qwen3"
            )
        })
        .await
        .expect("the agent must report the attention-only architecture");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
