use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_request_params_fixture::sample_decision::sample_decision;

#[tokio::test(flavor = "multi_thread")]
async fn a_buffered_decision_is_refused_once_its_agent_settles_without_decisions() {
    let mut cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams::default())
            .await
            .expect("a cluster whose agent downloads its model must start");
    let mut decision_messages = cluster_with_a_downloading_agent
        .cluster
        .client_inference
        .post_decide(CancellationToken::new(), &sample_decision())
        .await
        .expect("a cluster whose agent may still serve decisions must buffer the decision");

    cluster_with_a_downloading_agent
        .cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the decision must wait for the downloading agent");
    cluster_with_a_downloading_agent
        .cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &BalancerDesiredState::default())
        .await
        .expect("the balancer must accept a desired state without a model");

    let message = decision_messages
        .next()
        .await
        .expect("the decision stream must yield a message")
        .expect("the decision stream message must be readable");

    assert!(matches!(
        message,
        Message::Error(ErrorEnvelope { error, .. }) if error == JsonRpcError {
            code: 503,
            description: "No agent serves or is loading a model that supports decisions"
                .to_owned(),
        }
    ));

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster and its model server must shut down cleanly");
}
