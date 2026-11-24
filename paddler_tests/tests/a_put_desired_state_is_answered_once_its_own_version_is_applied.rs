use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::start_cluster::start_cluster;

const MODEL_URI: &str = "agent:///model.gguf";

#[tokio::test(flavor = "multi_thread")]
async fn a_put_desired_state_is_answered_once_its_own_version_is_applied() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");

    let desired_state = BalancerDesiredState {
        model: AgentDesiredModel::Uri(MODEL_URI.to_owned()),
        ..BalancerDesiredState::default()
    };

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &desired_state)
        .await
        .expect("the balancer must accept the desired state");

    assert_eq!(
        cluster
            .client_management
            .get_balancer_applicable_state(CancellationToken::new())
            .await
            .expect("the balancer must report its applicable state"),
        AgentDesiredState::from(&desired_state)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
