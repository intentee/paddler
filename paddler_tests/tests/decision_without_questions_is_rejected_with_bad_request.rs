use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn decision_without_questions_is_rejected_with_bad_request() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Decision))
        .await
        .expect("the cluster must start");

    let decide_url = cluster
        .balancer
        .inference_base_url()
        .expect("the balancer must expose its inference address")
        .join(ApiPath::DECIDE)
        .expect("the decide path must extend the inference address");
    let rejection = cluster
        .decide(
            CancellationToken::new(),
            &RawDecideParams {
                questions: Vec::new(),
                state: "state".to_owned(),
            },
        )
        .await
        .expect_err("a decision without questions must be rejected");

    assert_eq!(
        rejection
            .downcast_ref::<ClientError>()
            .map(ToString::to_string),
        Some(
            ClientError::UnexpectedResponseStatus {
                message: "Invalid request parameters: a decision needs at least one question"
                    .to_owned(),
                status: StatusCode::BAD_REQUEST,
                url: decide_url.to_string(),
            }
            .to_string()
        )
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
