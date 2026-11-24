#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use tokio::spawn;

use paddler_cli_tests::pausable_agent_cluster::PausableAgentCluster;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_cluster_harness::noul_system_one_request::noul_system_one_request;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_an_agent_killed_mid_decision_as_a_bad_gateway() {
    let PausableAgentCluster {
        mut cluster,
        pausable_agent,
    } = PausableAgentCluster::start(PausableAgentClusterParams {
        binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
        desired_state: ClusterDesiredState::Apply(Box::new(kev_0_8b().into_desired_state())),
        expected_slots_total: DECISION_SLOTS_MINIMUM.get(),
        slot_count: DECISION_SLOTS_MINIMUM.get(),
    })
    .await
    .expect("a decision cluster must start");

    pausable_agent
        .signals
        .pause()
        .expect("the agent must stop before the decision reaches it");

    let decision = spawn(cluster.typesafe_system_one("killed-request", &noul_system_one_request()));

    cluster
        .wait_for_slots_processing(&pausable_agent.id, 1)
        .await
        .expect("the balancer must dispatch the decision to the paused agent");
    pausable_agent
        .signals
        .kill()
        .expect("the agent must be killed while it holds the decision");

    assert_eq!(
        decision
            .await
            .expect("the decision task must not panic")
            .expect("the decision must be answered")
            .status,
        StatusCode::BAD_GATEWAY
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
