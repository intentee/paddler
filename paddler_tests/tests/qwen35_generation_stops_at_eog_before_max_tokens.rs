#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::generation_finish::GenerationFinish;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::greeting_conversation::greeting_conversation;
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;

#[tokio::test(flavor = "multi_thread")]
async fn qwen35_generation_stops_at_eog_before_max_tokens() {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(1)], false)
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &greeting_conversation(NonZeroU32::new(500).unwrap()),
        )
        .await
        .expect("the inference request must be accepted");

    let token_count = collected
        .token_results
        .iter()
        .filter(|TokenResultWithProducer { token_result, .. }| token_result.is_token())
        .count();

    assert!(token_count > 0);
    assert!(
        token_count < 500,
        "expected EOG to stop generation before max_tokens"
    );
    assert_eq!(
        collected
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::EndOfGeneration
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
