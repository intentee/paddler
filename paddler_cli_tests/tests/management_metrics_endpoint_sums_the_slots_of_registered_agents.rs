#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_cli_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_test_cluster_harness::agent_config::AgentConfig;

#[tokio::test(flavor = "multi_thread")]
async fn management_metrics_endpoint_sums_the_slots_of_registered_agents() {
    let cluster = start_subprocess_cluster_with_qwen3(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        vec![
            AgentConfig {
                name: "agent-with-one-slot".to_owned(),
                slot_count: 1,
            },
            AgentConfig {
                name: "agent-with-two-slots".to_owned(),
                slot_count: 2,
            },
        ],
    )
    .await
    .expect("the cluster must start");

    let metrics = cluster
        .client_management
        .get_metrics(CancellationToken::new())
        .await
        .expect("the metrics endpoint must answer");

    assert!(
        metrics.lines().any(|line| line == "paddler_slots_total 3"),
        "the slots of both agents must be summed: {metrics}"
    );
    assert!(
        metrics
            .lines()
            .any(|line| line == "paddler_slots_processing 0"),
        "idle agents must process no slots: {metrics}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
