#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_decision::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::start_decision_cluster::start_decision_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_too_few_slots_for_decisions() {
    let mut cluster = start_decision_cluster(DecisionClusterParams {
        agents: vec![AgentConfig::single(1)],
        wait_for_slots_ready: false,
        ..DecisionClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            *issue
                == AgentIssue::SlotsInsufficientForDecisions(SlotsInsufficientForDecisionsParams {
                    desired_slots: 1,
                    required_slots: DECISION_SLOTS_MINIMUM,
                })
        })
        .await
        .expect("the agent must report too few slots");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
