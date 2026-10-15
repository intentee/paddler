use std::io;
use std::io::ErrorKind;

use tokio_util::sync::CancellationToken;

use paddler_cli_tests::missing_binary_path::MISSING_BINARY_PATH;
use paddler_cli_tests::signalled_subprocess::SignalledSubprocess;
use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_cli_tests::subprocess_agent_spawner::SubprocessAgentSpawner;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_reports_an_agent_binary_that_cannot_be_spawned() {
    let ephemeral_loopback_addr = EPHEMERAL_LOOPBACK_ADDR.to_string();
    let running_balancer = spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        [
            "--inference-addr",
            &ephemeral_loopback_addr,
            "--management-addr",
            &ephemeral_loopback_addr,
        ],
        SignalledSubprocess::new,
    )
    .await
    .expect("the balancer must start and announce its addresses")
    .running_balancer;
    let management_addr = running_balancer.addresses.management;
    let mut cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(SubprocessAgentSpawner::new(
            MISSING_BINARY_PATH.to_owned(),
            management_addr,
        )),
        &ClusterDesiredState::Apply(Box::default()),
    )
    .await
    .expect("the cluster must connect to the balancer");

    let registration_error = cluster
        .register_agents(&[AgentConfig::single(1)], false)
        .await
        .expect_err("an agent binary that is missing must not register");

    assert_eq!(
        registration_error
            .root_cause()
            .downcast_ref::<io::Error>()
            .map(io::Error::kind),
        Some(ErrorKind::NotFound)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
