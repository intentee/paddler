use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

use paddler_cli_tests::pausable_agent_cluster::PausableAgentCluster;
use paddler_cli_tests::pausable_agent_cluster_params::PausableAgentClusterParams;
use paddler_cli_tests::start_pausable_agent_cluster::start_pausable_agent_cluster;
use paddler_client::error::Error as ClientError;
use paddler_messaging::api_path::ApiPath;
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

    let model_metadata_url = cluster
        .balancer
        .management_base_url
        .join(&ApiPath::agent_model_metadata(&pausable_agent.id))
        .expect("the model metadata path must extend the management address");
    let metadata_result = cluster
        .client_management
        .get_model_metadata(CancellationToken::new(), &pausable_agent.id)
        .await;

    assert_eq!(
        metadata_result.err().as_ref().map(ToString::to_string),
        Some(
            ClientError::UnexpectedResponseStatus {
                message: String::new(),
                status: StatusCode::GATEWAY_TIMEOUT,
                url: model_metadata_url.to_string(),
            }
            .to_string()
        )
    );

    pausable_agent
        .signals
        .resume()
        .expect("the agent must continue so that it can shut down");
    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
