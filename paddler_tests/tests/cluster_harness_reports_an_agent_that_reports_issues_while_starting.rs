use std::collections::BTreeSet;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

const AGENT_NAME: &str = "agent-without-its-model";
const MISSING_MODEL_PATH: &str = "/nonexistent/model.gguf";

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_reports_an_agent_that_reports_issues_while_starting() {
    let startup_error = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: AGENT_NAME.to_owned(),
            slot_count: 1,
        }],
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model: AgentDesiredModel::LocalToAgent(MISSING_MODEL_PATH.to_owned()),
            ..BalancerDesiredState::default()
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .err()
    .expect("a cluster whose agent cannot load its model must fail to start");

    assert!(matches!(
        startup_error.downcast_ref::<ClusterHarnessError>(),
        Some(ClusterHarnessError::AgentReportedIssues { agent_name, issues })
            if agent_name == AGENT_NAME
                && *issues == BTreeSet::from([AgentIssue::ModelFileDoesNotExist(ModelPath {
                    model_path: MISSING_MODEL_PATH.to_owned(),
                })])
    ));
}
