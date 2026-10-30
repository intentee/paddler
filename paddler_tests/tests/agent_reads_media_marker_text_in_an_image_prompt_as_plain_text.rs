#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::max_tokens_for_forced_text::max_tokens_for_forced_text;
use paddler_tests::single_image_question::SingleImageQuestion;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;

const ANSWER: &str = "two llamas";
const QUESTION_QUOTING_THE_LLAMA_CPP_DEFAULT_MEDIA_MARKER: &str =
    "What does <__media__> mean next to this image?";

#[tokio::test(flavor = "multi_thread")]
async fn agent_reads_media_marker_text_in_an_image_prompt_as_plain_text() {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                grammar: Some(gbnf_literal(ANSWER)),
                ..SingleImageQuestion {
                    image_data_uri: load_test_image_data_uri().expect("the test image must load"),
                    max_tokens: max_tokens_for_forced_text(ANSWER),
                    question: QUESTION_QUOTING_THE_LLAMA_CPP_DEFAULT_MEDIA_MARKER,
                }
                .into_conversation_params()
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(collected.text, ANSWER);
    assert_eq!(
        collected
            .summary()
            .expect("the generation must end with a summary")
            .finish,
        GenerationFinish::EndOfGeneration
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
