use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn management_serves_the_applicable_state_of_the_desired_state() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    let agent_desired_state = cluster
        .client_management
        .get_balancer_applicable_state(CancellationToken::new())
        .await
        .expect("the balancer must serve its applicable state");

    assert_eq!(
        agent_desired_state,
        AgentDesiredState::from(BalancerDesiredState::unconfigured(
            InferenceMode::TextGeneration
        ))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
