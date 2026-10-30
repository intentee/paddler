use std::io;
use std::io::ErrorKind;

use paddler_test_cluster_harness::agent_config::AgentConfig;

use crate::cluster_with_a_missing_agent_binary::cluster_with_a_missing_agent_binary;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_reports_an_agent_binary_that_cannot_be_spawned() {
    let mut cluster = cluster_with_a_missing_agent_binary().await;

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
