use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_balancer_tests::conversation_with_a_tool_requiring_an_undeclared_property::conversation_with_a_tool_requiring_an_undeclared_property;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::inference_client::message::Message as InferenceClientMessage;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_rejects_a_tool_that_requires_an_undeclared_property() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");

    let answer = cluster
        .client_inference
        .continue_from_conversation_history(
            CancellationToken::new(),
            conversation_with_a_tool_requiring_an_undeclared_property(),
        )
        .await
        .expect("the request must be sent")
        .next()
        .await
        .expect("the request must be answered")
        .expect("the answer must arrive over a connection that stays open");

    assert!(matches!(
        answer,
        InferenceClientMessage::Error(envelope)
            if envelope.error.code == 400
                && envelope.error.description
                    == "Invalid request parameters: Required field 'nonexistent_field' not found in properties"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
