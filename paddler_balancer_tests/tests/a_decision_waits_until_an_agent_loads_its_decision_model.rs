#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;
use paddler_cluster_harness::decision_done_discriminant::decision_done_discriminant;
use paddler_messaging::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;
use paddler_request_params_fixture::sample_decision::sample_decision;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_waits_until_an_agent_loads_its_decision_model() {
    let mut cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams {
            agents: AgentConfig::uniform(1, DECISION_SLOTS_WITH_A_QUESTION_LANE.get()),
            ..ClusterParams::default()
        })
        .await
        .expect("a cluster whose agent downloads its model must start");
    let cluster = &mut cluster_with_a_downloading_agent.cluster;

    let decision = spawn(cluster.decide(CancellationToken::new(), &sample_decision()));

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the decision must wait in the buffer");
    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &DecisionDesiredState::default().into_balancer_desired_state(),
        )
        .await
        .expect("the balancer must accept the decision model");

    assert_eq!(
        discriminant(
            &decision
                .await
                .expect("the decision task must not panic")
                .expect("the buffered decision must be answered")
                .terminal_result
        ),
        decision_done_discriminant()
    );

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
