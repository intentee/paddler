use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_without_a_pointer_head_refuses_decisions_as_unavailable() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer without a pointer head must start");

    let decide_url = cluster
        .balancer
        .inference_base_url
        .join(ApiPath::DECIDE)
        .expect("the decide path must extend the inference address");
    let decision_rejection = cluster
        .client_inference
        .post_decide(
            CancellationToken::new(),
            &RawDecideParams {
                questions: vec![DecisionQuestion {
                    id: "paid".to_owned(),
                    instructions: "Was it paid?".to_owned(),
                    options: vec!["no".to_owned(), "yes".to_owned()],
                }],
                state: "The invoice was paid on time.".to_owned(),
            },
        )
        .await
        .err()
        .expect("a cluster without a pointer head must refuse a decision");

    assert_eq!(
        decision_rejection.to_string(),
        ClientError::ServiceUnavailable {
            message: "No pointer head is configured, so the cluster serves no decisions".to_owned(),
            url: decide_url.to_string(),
        }
        .to_string()
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
