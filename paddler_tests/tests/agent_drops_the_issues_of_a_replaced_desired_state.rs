use std::collections::BTreeSet;

use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

#[tokio::test(flavor = "multi_thread")]
async fn agent_drops_the_issues_of_a_replaced_desired_state() {
    let replaced_model_uri = "not a valid uri".to_owned();
    let replacement_model_uri = "https://huggingface.co/owner/repo".to_owned();
    let mut cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: AgentDesiredModel::Uri(replaced_model_uri.clone()),
        ..BalancerDesiredState::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ModelUriIsUnparseable(model_path) if model_path.model_path == replaced_model_uri)
        })
        .await
        .expect("the agent must report the issue of the replaced desired state");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                model: AgentDesiredModel::Uri(replacement_model_uri.clone()),
                ..BalancerDesiredState::default()
            },
        )
        .await
        .expect("the balancer must apply the replacement desired state");

    let snapshot = cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::HuggingFaceModelUriIsMalformed(model_path) if model_path.model_path == replacement_model_uri)
        })
        .await
        .expect("the agent must report the issue of the replacement desired state");

    assert_eq!(
        snapshot.agents[0].status.issues,
        BTreeSet::from([AgentIssue::HuggingFaceModelUriIsMalformed(ModelPath {
            model_path: replacement_model_uri,
        })])
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
