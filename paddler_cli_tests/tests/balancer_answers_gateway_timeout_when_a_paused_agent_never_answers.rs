use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

use paddler_cli_tests::pausable_agent_cluster::PausableAgentCluster;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_client::error::Error as ClientError;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_answers_gateway_timeout_when_a_paused_agent_never_answers() {
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

    let metadata_result = cluster
        .client_management
        .get_model_metadata(CancellationToken::new(), &pausable_agent.id)
        .await;

    assert!(matches!(
        metadata_result,
        Err(ClientError::UnexpectedResponseStatus { status, .. }) if status == StatusCode::GATEWAY_TIMEOUT
    ));

    pausable_agent
        .signals
        .resume()
        .expect("the agent must continue so that it can shut down");
    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
