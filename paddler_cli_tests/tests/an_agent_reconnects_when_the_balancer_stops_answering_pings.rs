use std::net::Ipv4Addr;
use std::net::SocketAddr;

use tokio::net::TcpListener;

use paddler_cli_tests::pausable_agent::PausableAgent;
use paddler_cli_tests::pausable_agent_params::PausableAgentParams;
use paddler_cli_tests::stallable_tcp_relay::StallableTcpRelay;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_reconnects_when_the_balancer_stops_answering_pings() {
    let mut cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams::without_agents(),
    )
    .await
    .expect("a cluster without agents must start");
    let relay_listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("the relay must bind a loopback port");
    let relay_addr = relay_listener
        .local_addr()
        .expect("the relay listener must know its address");
    let relay = StallableTcpRelay::start(relay_listener, cluster.balancer.addresses.management);

    PausableAgent::join(
        &mut cluster,
        PausableAgentParams {
            binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
            config: AgentConfig::single(1),
            expected_slots_total: 0,
            management_addr: relay_addr,
        },
    )
    .await
    .expect("the agent must register through the relay");

    relay.stall_established_connections();
    relay
        .wait_for_accepted_connections(2)
        .await
        .expect("the agent must reconnect once the balancer stops answering its pings");

    relay
        .stop()
        .await
        .expect("the relay task must not panic")
        .expect("the relay must stop without a relaying failure");
    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
