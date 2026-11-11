use std::io;
use std::io::ErrorKind;

use paddler_cli_tests::cluster_with_a_missing_agent_binary::cluster_with_a_missing_agent_binary;
use paddler_test_cluster_harness::agent_config::AgentConfig;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_reports_an_additional_agent_binary_that_cannot_be_spawned() {
    let mut cluster =
        cluster_with_a_missing_agent_binary(env!("CARGO_BIN_EXE_paddler_cluster_node"))
            .await
            .expect("the cluster must connect to the balancer it started");

    assert_eq!(
        cluster
            .spawn_additional_agent(&AgentConfig::single(1))
            .err()
            .as_ref()
            .and_then(|spawn_error| spawn_error.root_cause().downcast_ref::<io::Error>())
            .map(io::Error::kind),
        Some(ErrorKind::NotFound)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
