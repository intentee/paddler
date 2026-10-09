#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;
use tokio::spawn;

use paddler_agent_decision::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_cli_tests::pausable_agent_cluster::PausableAgentCluster;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_test_cluster_harness::noul_system_one_request::noul_system_one_request;
use paddler_test_cluster_harness::pointer_head_fixture::pointer_head_fixture;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_reports_an_agent_killed_mid_decision_as_a_bad_gateway() {
    let PausableAgentCluster {
        mut cluster,
        pausable_agent,
    } = start_pausable_agent_cluster(PausableAgentClusterParams {
        binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
        desired_state: ClusterDesiredState::Apply(Box::new(
            qwen3_5_0_8b().into_decision_desired_state(pointer_head_fixture(
                QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE,
            )),
        )),
        expected_slots_total: DECISION_SLOTS_MINIMUM,
        slot_count: DECISION_SLOTS_MINIMUM,
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
