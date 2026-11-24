use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::agent_model_uri_not_canonical_params::AgentModelUriNotCanonicalParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_an_agent_uri_that_is_not_canonical() {
    let model_uri = "agent:///models/my model.gguf".to_owned();
    let expected_issue = AgentIssue::AgentModelUriIsNotCanonical(AgentModelUriNotCanonicalParams {
        canonical_uri: "agent:///models/my%20model.gguf".to_owned(),
        model_path: ModelPath {
            model_path: model_uri.clone(),
        },
    });
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::Uri(model_uri),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| *issue == expected_issue)
        .await
        .expect("the agent must report the canonical form of the agent URI");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
