#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use crate::subprocess_cluster_with_a_serving_and_a_stuck_agent::SERVING_AGENT_SLOTS;
use crate::subprocess_cluster_with_a_serving_and_a_stuck_agent::subprocess_cluster_with_a_serving_and_a_stuck_agent;

#[tokio::test(flavor = "multi_thread")]
async fn management_metrics_count_only_the_slots_of_serving_agents() {
    let cluster = subprocess_cluster_with_a_serving_and_a_stuck_agent().await;

    let metrics = cluster
        .client_management
        .get_metrics(CancellationToken::new())
        .await
        .expect("the metrics endpoint must answer");

    assert!(
        metrics
            .lines()
            .any(|line| line == format!("paddler_slots_total {SERVING_AGENT_SLOTS}"))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
