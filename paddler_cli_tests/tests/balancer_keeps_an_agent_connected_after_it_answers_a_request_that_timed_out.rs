use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

use paddler_cli_tests::pausable_agent_cluster::PausableAgentCluster;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_client::error::Error as ClientError;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_keeps_an_agent_connected_after_it_answers_a_request_that_timed_out() {
    let PausableAgentCluster {
        cluster,
        pausable_agent,
    } = start_pausable_agent_cluster(PausableAgentClusterParams {
        binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
        desired_state: ClusterDesiredState::KeepStored,
        expected_slots_total: 0,
        slot_count: 1,
    })
    .await
    .expect("a cluster with a model-less agent must start");

    pausable_agent
        .signals
        .pause()
        .expect("the agent must stop before it is asked anything");

    assert!(matches!(
        cluster
            .client_management
            .get_model_metadata(CancellationToken::new(), &pausable_agent.id)
            .await,
        Err(ClientError::UnexpectedResponseStatus { status, .. }) if status == StatusCode::GATEWAY_TIMEOUT
    ));

    pausable_agent
        .signals
        .resume()
        .expect("the agent must continue and answer the request that already timed out");

    assert_eq!(
        cluster
            .client_management
            .get_model_metadata(CancellationToken::new(), &pausable_agent.id)
            .await
            .expect("the balancer must relay the answer of the agent that stayed connected"),
        None
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
