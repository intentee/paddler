use reqwest::Client;
use reqwest::StatusCode;

use paddler_balancer_typesafe_service::constants::TYPESAFE_API_PATH_SYSTEM_ONE;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_answers_a_body_that_is_not_json_as_unprocessable() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");

    let not_json = Client::new()
        .post(
            cluster
                .balancer
                .compat_typesafe_url(TYPESAFE_API_PATH_SYSTEM_ONE)
                .expect("the cluster serves TypeSafe"),
        )
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .expect("the request must be answered");

    assert_eq!(not_json.status(), StatusCode::UNPROCESSABLE_ENTITY);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
