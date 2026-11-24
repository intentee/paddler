use paddler_balancer_typesafe_service::constants::TYPESAFE_HEADER_REQUEST_ID;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_models_responds_with_a_request_id() {
    let cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams::default())
            .await
            .expect("a cluster whose agent downloads its model must start");
    let cluster = &cluster_with_a_downloading_agent.cluster;
    let response = cluster
        .typesafe_models()
        .await
        .expect("the models request must be answered");

    assert!(
        response
            .headers
            .get(TYPESAFE_HEADER_REQUEST_ID)
            .is_some_and(|request_id| !request_id.is_empty())
    );

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
