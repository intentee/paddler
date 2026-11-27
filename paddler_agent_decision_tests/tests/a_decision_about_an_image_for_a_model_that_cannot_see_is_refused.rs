#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings_types::decision_request::DecisionRequest;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision_tests::snowy_weather_decision::snowy_weather_decision;
use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_file_fixture::image_fixture::ImageFixture;
use paddler_messaging::decision_result::DecisionResult;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_about_an_image_for_a_model_that_cannot_see_is_refused() {
    let cluster = DecisionClusterParams::default()
        .start_cluster()
        .await
        .expect("the cluster must start");

    let collected = cluster
        .decide(
            CancellationToken::new(),
            &DecisionRequest {
                images: vec![
                    ImageFixture::LlamasJpeg
                        .data_uri()
                        .expect("the test image must load"),
                ],
                ..snowy_weather_decision()
            },
        )
        .await
        .expect("the decision must be answered with a refusal");

    assert_eq!(collected.answers, Vec::new());
    assert!(matches!(
        collected.terminal_result,
        DecisionResult::RequestRejected(_)
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
