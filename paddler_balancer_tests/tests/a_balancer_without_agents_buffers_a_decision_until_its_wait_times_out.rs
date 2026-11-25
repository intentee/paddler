use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::constants::SHORT_BUFFERED_REQUEST_TIMEOUT;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_request_params_fixture::sample_decision::sample_decision;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_without_agents_buffers_a_decision_until_its_wait_times_out() {
    let cluster = Cluster::start_in_process(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        max_buffered_requests: 1,
        ..ClusterParams::without_agents()
    })
    .await
    .expect("a cluster without agents must start");

    let message = cluster
        .client_inference
        .post_decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("a cluster without agents must buffer the decision")
        .next()
        .await
        .expect("the decision stream must yield a message")
        .expect("the decision stream message must be readable");

    assert!(matches!(
        message,
        Message::Error(ErrorEnvelope { error, .. }) if error == JsonRpcError {
            code: 504,
            description: "Waiting for available slot timed out".to_owned(),
        }
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
