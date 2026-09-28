use futures_util::StreamExt as _;
use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::conversation_with_a_tool_requiring_an_undeclared_property::conversation_with_a_tool_requiring_an_undeclared_property;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_rejects_tool_whose_required_field_is_not_a_property() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
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
