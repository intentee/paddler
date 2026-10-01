#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::image_description_conversation::image_description_conversation;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;

const FIRST_ANSWER: &str = "The first picture.";
const SECOND_ANSWER: &str = "The second picture.";

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_two_concurrent_multimodal_requests_produce_tokens() {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(4)])
        .await
        .expect("the cluster must start");

    let image_data_uri = load_test_image_data_uri().expect("the test image must load");

    let first_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: image_description_conversation(image_data_uri.clone()),
        enable_thinking: false,
        grammar: Some(gbnf_literal(FIRST_ANSWER)),
        max_tokens: NonZeroU32::new(32).unwrap(),
        parse_tool_calls: false,
        tools: vec![],
    };
    let second_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: image_description_conversation(image_data_uri),
        enable_thinking: false,
        grammar: Some(gbnf_literal(SECOND_ANSWER)),
        max_tokens: NonZeroU32::new(32).unwrap(),
        parse_tool_calls: false,
        tools: vec![],
    };
    let (first_collected, second_collected) = join!(
        cluster.continue_from_conversation_history(CancellationToken::new(), &first_params),
        cluster.continue_from_conversation_history(CancellationToken::new(), &second_params),
    );

    let first_collected = first_collected.expect("the first request must complete");
    let second_collected = second_collected.expect("the second request must complete");

    assert_eq!(first_collected.text, FIRST_ANSWER);
    assert_eq!(
        first_collected
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::EndOfGeneration
    );
    assert_eq!(second_collected.text, SECOND_ANSWER);
    assert_eq!(
        second_collected
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
