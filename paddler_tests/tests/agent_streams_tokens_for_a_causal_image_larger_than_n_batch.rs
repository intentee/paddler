#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::jpeg_and_webp_image_fixtures::JPEG_AND_WEBP_IMAGE_FIXTURES;
use paddler_tests::single_image_question::SingleImageQuestion;
use paddler_tests::start_cluster_with_smolvlm2_and_n_batch::start_cluster_with_smolvlm2_and_n_batch;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(20).unwrap();
const N_BATCH_SMALLER_THAN_ONE_IMAGE: u32 = 32;

#[tokio::test(flavor = "multi_thread")]
async fn agent_streams_tokens_for_a_causal_image_larger_than_n_batch() {
    let cluster = start_cluster_with_smolvlm2_and_n_batch(
        vec![AgentConfig::single(1)],
        N_BATCH_SMALLER_THAN_ONE_IMAGE,
    )
    .await
    .expect("the cluster must start");

    for image_fixture in JPEG_AND_WEBP_IMAGE_FIXTURES {
        let collected = cluster
            .continue_from_conversation_history(
                CancellationToken::new(),
                &SingleImageQuestion {
                    image_data_uri: image_fixture
                        .data_uri()
                        .expect("the image fixture must load"),
                    max_tokens: MAX_TOKENS,
                    question: "Describe this image.",
                }
                .into_conversation_params(),
            )
            .await
            .expect("the image request must stream tokens");

        assert!(
            collected
                .token_results
                .iter()
                .any(|token_result_with_producer| token_result_with_producer
                    .token_result
                    .is_token()),
            "fixture {} must stream tokens: {:?}",
            image_fixture.fixture_name,
            collected.token_results
        );
        assert!(
            collected
                .summary()
                .expect("the generation must report its summary")
                .usage
                .input_image_tokens
                > u64::from(N_BATCH_SMALLER_THAN_ONE_IMAGE)
        );
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
