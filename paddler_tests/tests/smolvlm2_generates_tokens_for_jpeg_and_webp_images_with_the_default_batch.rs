#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::jpeg_and_webp_image_fixtures::JPEG_AND_WEBP_IMAGE_FIXTURES;
use paddler_tests::single_image_question::SingleImageQuestion;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(20).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn smolvlm2_generates_tokens_for_jpeg_and_webp_images_with_the_default_batch() {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    for image_fixture in JPEG_AND_WEBP_IMAGE_FIXTURES {
        let token_results = cluster
            .continue_from_conversation_history(
                CancellationToken::new(),
                &SingleImageQuestion {
                    image_data_uri: image_fixture
                        .data_uri()
                        .expect("the image fixture must load"),
                    max_tokens: MAX_TOKENS,
                    question: "What do you see in this image?",
                }
                .into_conversation_params(),
            )
            .await
            .expect("the image fixture request must complete")
            .into_token_results();

        assert!(
            token_results.iter().any(GeneratedTokenResult::is_token),
            "fixture {} should produce at least one content/reasoning/tool-call token with adequate n_batch; got {token_results:?}",
            image_fixture.fixture_name,
        );
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
