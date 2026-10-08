use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

const BAD_REQUEST: u16 = 400;

#[tokio::test(flavor = "multi_thread")]
async fn decision_without_questions_is_rejected_with_bad_request() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Decision))
        .await
        .expect("the cluster must start");

    let rejection = cluster
        .client_inference
        .post_decide(
            CancellationToken::new(),
            &RawDecideParams {
                questions: Vec::new(),
                state: "state".to_owned(),
            },
        )
        .await
        .err();

    assert!(matches!(
        rejection,
        Some(ClientError::UnexpectedResponseStatus { message, status, .. })
            if status.as_u16() == BAD_REQUEST
                && message == "Invalid request parameters: a decision needs at least one question"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
