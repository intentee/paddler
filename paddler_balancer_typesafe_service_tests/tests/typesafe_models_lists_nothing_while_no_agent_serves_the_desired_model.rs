use reqwest::StatusCode;
use serde_json::json;

use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::cluster_with_an_agent_downloading_its_model::ClusterWithAnAgentDownloadingItsModel;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_models_lists_nothing_while_no_agent_serves_the_desired_model() {
    let cluster_with_a_downloading_agent =
        ClusterWithAnAgentDownloadingItsModel::start(ClusterParams::default())
            .await
            .expect("a cluster whose agent downloads its model must start");
    let cluster = &cluster_with_a_downloading_agent.cluster;
    let response = cluster
        .typesafe_models()
        .await
        .expect("the models request must be answered");

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, json!({"models": []}));

    cluster_with_a_downloading_agent
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
