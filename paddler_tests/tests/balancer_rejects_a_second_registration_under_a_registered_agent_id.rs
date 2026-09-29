use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_a_second_registration_under_a_registered_agent_id() {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: Some(BalancerDesiredState::default()),
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
    let (mut impostor_socket, _handshake_response) = connect_async(format!(
        "ws://{}/api/v1/agent_socket/{registered_agent_id}",
        cluster.balancer.addresses.management
    ))
    .await
    .expect("the impostor must reach the agent socket");

    impostor_socket
        .send(Message::text(
            serde_json::to_string(&ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                    name: Some("impostor".to_owned()),
                    slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot::default(),
                }),
            ))
            .expect("the registration must serialize"),
        ))
        .await
        .expect("the impostor registration must be sent");

    let close_frame = loop {
        if let Message::Close(close_frame) = impostor_socket
            .next()
            .await
            .expect("the balancer must close the impostor socket")
            .expect("the impostor socket must stay readable until it is closed")
        {
            break close_frame;
        }
    };

    assert_eq!(
        close_frame.map(|close_frame| close_frame.code),
        Some(CloseCode::Policy)
    );
    assert!(
        cluster
            .client_management
            .get_model_metadata(CancellationToken::new(), &registered_agent_id)
            .await
            .is_ok(),
        "the registered agent must keep serving requests"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
