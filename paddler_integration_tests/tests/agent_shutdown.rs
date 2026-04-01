#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::os::unix::process::ExitStatusExt;

use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use serial_test::file_serial;

#[tokio::test]
#[file_serial]
async fn test_agent_shuts_down_gracefully() {
    let mut cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "shutdown-test-agent".to_string(),
        agent_slots: 1,
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let exit_status = cluster
        .agent
        .graceful_shutdown()
        .await
        .expect("failed to send SIGTERM to agent");

    assert!(
        exit_status.signal().is_none(),
        "Agent was killed by signal {:?} instead of exiting cleanly",
        exit_status.signal()
    );

    assert!(
        exit_status.success(),
        "Agent exited with non-zero status: {exit_status}"
    );

    let agent_count = cluster.balancer.wait_for_agent_count(0).await;

    assert_eq!(
        agent_count, 0,
        "Balancer should see zero agents after agent shutdown"
    );
}
