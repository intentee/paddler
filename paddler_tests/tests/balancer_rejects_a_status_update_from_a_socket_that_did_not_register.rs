use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::untrusted_agent_socket_client::UntrustedAgentSocketClient;
use paddler_tests::start_cluster::start_cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_a_status_update_from_a_socket_that_did_not_register() {
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
    let mut impostor_socket = UntrustedAgentSocketClient::connect(
        cluster.balancer.addresses.management,
        &registered_agent_id,
    )
    .await
    .expect("the impostor must reach the agent socket");

    impostor_socket
        .send_forged_notification(ManagementJsonRpcNotification::UpdateAgentStatus(
            UpdateAgentStatusParams {
                slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot {
                    status: AgentStatus {
                        desired_slots_total: 99,
                        ..AgentStatus::default()
                    },
                    version: u64::MAX,
                },
            },
        ))
        .await
        .expect("the impostor status update must be sent");

    assert_eq!(
        impostor_socket
            .next_close_frame()
            .await
            .expect("the balancer must close the impostor socket")
            .map(|close_frame| close_frame.code),
        Some(CloseCode::Policy)
    );

    let registered_agents = cluster
        .client_management
        .get_agents(CancellationToken::new())
        .await
        .expect("the balancer must list its agents")
        .agents;

    assert_eq!(
        registered_agents
            .iter()
            .map(|agent| agent.status.desired_slots_total)
            .collect::<Vec<_>>(),
        vec![1]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
