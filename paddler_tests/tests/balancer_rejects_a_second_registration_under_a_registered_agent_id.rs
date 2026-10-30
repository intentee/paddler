use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_util::sync::CancellationToken;

use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

use crate::untrusted_management_agent::UntrustedManagementAgent;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_a_second_registration_under_a_registered_agent_id() {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::default()),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a single-agent cluster must start");
    let registered_agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have a registered agent")
        .clone();
    let mut impostor_agent = UntrustedManagementAgent::connect(
        cluster.balancer.addresses.management,
        &registered_agent_id,
    )
    .await
    .expect("the impostor must reach the agent socket");

    impostor_agent
        .register(RegisterAgentParams {
            name: Some("impostor".to_owned()),
            slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot::default(),
        })
        .await
        .expect("the impostor registration must be sent");

    assert_eq!(
        impostor_agent
            .socket_client
            .next_close_frame()
            .await
            .expect("the balancer must close the impostor socket")
            .map(|close_frame| close_frame.code),
        Some(CloseCode::Policy)
    );
    assert_eq!(
        cluster
            .client_management
            .get_model_metadata(CancellationToken::new(), &registered_agent_id)
            .await
            .expect("the registered agent must keep serving requests"),
        None
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
