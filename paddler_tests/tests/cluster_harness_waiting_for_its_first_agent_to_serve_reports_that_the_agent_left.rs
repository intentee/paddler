use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_waiting_for_its_first_agent_to_serve_reports_that_the_agent_left() {
    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::default()),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a single-agent cluster must start");
    let awaited_agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have a registered agent")
        .clone();

    cluster
        .agents
        .pop()
        .expect("the cluster must have its registered agent")
        .shutdown()
        .await
        .expect("the agent must leave the pool cleanly");

    assert!(matches!(
        cluster
            .wait_for_first_agent_to_serve(AgentDecisionsServing::NotServed)
            .await
            .err(),
        Some(ClusterHarnessError::AgentLeftThePool { agent_id }) if agent_id == awaited_agent_id
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
