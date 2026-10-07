#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
use paddler_messaging::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_pointer_head_delimiters_that_are_not_control_tokens() {
    let mut cluster = start_decision_cluster(DecisionClusterParams {
        pointer_head_fixture: "qwen3_5_0_8b_pointer_head_with_plain_text_delimiters.gguf",
        wait_for_slots_ready: false,
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(
                issue,
                AgentIssue::PointerHeadIncompatibleWithModel(PointerHeadIncompatibleWithModelParams {
                    incompatibility: PointerHeadIncompatibility::DelimiterIsNotAControlToken {
                        delimiter,
                    },
                    ..
                }) if delimiter == "decide"
            )
        })
        .await
        .expect("the agent must report the plain text delimiter");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
