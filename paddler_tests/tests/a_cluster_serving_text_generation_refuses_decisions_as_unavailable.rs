use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_serving_text_generation_refuses_decisions_as_unavailable() {
    let cluster = start_cluster(cluster_without_agents_serving(
        InferenceMode::TextGeneration,
    ))
    .await
    .expect("a balancer serving text generation must start");

    let decide_url = cluster
        .balancer
        .inference_base_url()
        .expect("the balancer must expose its inference address")
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
        .expect("a text generation cluster must refuse a decision");

    assert_eq!(
        decision_rejection.to_string(),
        ClientError::ServiceUnavailable {
            message: "The cluster serves TextGeneration, not Decision".to_owned(),
            url: decide_url.to_string(),
        }
        .to_string()
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
