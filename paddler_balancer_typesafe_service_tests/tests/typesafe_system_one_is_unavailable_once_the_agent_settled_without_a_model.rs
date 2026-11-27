use reqwest::StatusCode;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::noul_system_one_request::noul_system_one_request;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_is_unavailable_once_the_agent_settled_without_a_model() {
    let mut cluster = Cluster::start_in_process(ClusterParams {
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster whose agent serves no model must start");

    cluster
        .wait_for_first_agent_to_apply_its_state()
        .await
        .expect("the agent must apply its state without a model");

    let response = cluster
        .typesafe_system_one(
            "text-generation-cluster-request",
            &noul_system_one_request(),
        )
        .await
        .expect("the System One request must be answered");

    assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
