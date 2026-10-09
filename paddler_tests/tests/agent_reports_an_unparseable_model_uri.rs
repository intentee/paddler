use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_an_unparseable_model_uri() {
    let model_uri = "not a valid uri".to_owned();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::Uri(model_uri.clone()),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ModelUriIsUnparseable(model_path) if model_path.model_path == model_uri)
        })
        .await
        .expect("the agent must report that the model URI cannot be parsed");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
