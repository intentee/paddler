use paddler_client::error::Error as ClientError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::conversation_with_a_tool_requiring_an_undeclared_property::conversation_with_a_tool_requiring_an_undeclared_property;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const BAD_REQUEST: u16 = 400;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_tool_whose_required_field_is_not_a_property() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    let rejection = cluster
        .client_inference
        .post_continue_from_conversation_history(
            CancellationToken::new(),
            &conversation_with_a_tool_requiring_an_undeclared_property(),
        )
        .await
        .err()
        .expect("the balancer must reject a tool whose required field is not a property");

    assert!(matches!(
        rejection,
        ClientError::UnexpectedResponseStatus { message, status, .. }
            if status.as_u16() == BAD_REQUEST
                && message == "Invalid request parameters: Required field 'nonexistent_field' not found in properties"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
