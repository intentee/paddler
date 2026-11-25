use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_the_url_model_of_its_desired_state() {
    let configured_url = "https://example.invalid/persisted-model.gguf".to_owned();

    let cluster = Cluster::start_in_process(ClusterParams {
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model: AgentDesiredModel::Uri(configured_url.clone()),
            ..BalancerDesiredState::default()
        })),
        ..ClusterParams::without_agents()
    })
    .await
    .expect("the cluster must start");

    let retrieved = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("failed to read balancer desired state");

    assert_eq!(retrieved.model, AgentDesiredModel::Uri(configured_url));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
