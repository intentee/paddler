use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_reports_an_awaited_agent_that_leaves_the_pool() {
    let Cluster {
        agent_ids,
        agents,
        mut agents_watcher,
        balancer,
        ..
    } = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a single-agent cluster must start");
    let awaited_agent_id = agent_ids
        .first()
        .expect("the cluster must have a registered agent")
        .clone();

    for agent in agents {
        agent
            .shutdown()
            .await
            .expect("the agent must shut down cleanly");
    }

    let observation = agents_watcher
        .until_agent(
            &awaited_agent_id,
            ObservationWindow::release(),
            |_snapshot| false,
        )
        .await;

    assert!(matches!(
        observation,
        Err(ClusterHarnessError::AgentLeftThePool { agent_id }) if agent_id == awaited_agent_id
    ));

    balancer
        .shutdown()
        .await
        .expect("the balancer must shut down cleanly");
}
