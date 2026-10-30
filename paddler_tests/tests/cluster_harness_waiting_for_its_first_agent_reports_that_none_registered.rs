use std::mem::discriminant;

use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_waiting_for_its_first_agent_reports_that_none_registered() {
    let mut cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");

    assert_eq!(
        cluster
            .wait_for_first_agent_to_serve(AgentDecisionsServing::NotServed)
            .await
            .err()
            .as_ref()
            .map(discriminant),
        Some(discriminant(
            &ClusterHarnessError::ClusterHasNoRegisteredAgent
        ))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
