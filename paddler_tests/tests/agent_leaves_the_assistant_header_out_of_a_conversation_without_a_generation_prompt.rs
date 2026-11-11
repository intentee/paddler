#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;

const QWEN3_ASSISTANT_HEADER_WITHOUT_THINKING: &str =
    "<|im_start|>assistant\n<think>\n\n</think>\n\n";

#[tokio::test(flavor = "multi_thread")]
async fn agent_leaves_the_assistant_header_out_of_a_conversation_without_a_generation_prompt() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");
    let prompt_tokens_of_the_weather_question = async |add_generation_prompt: bool| -> u64 {
        cluster
            .continue_from_conversation_history(
                CancellationToken::new(),
                &ContinueFromConversationHistoryParams {
                    add_generation_prompt,
                    ..weather_question_conversation(gbnf_literal("a"), NonZeroU32::MIN, vec![])
                },
            )
            .await
            .expect("the inference request must be accepted")
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .prompt
    };
    let assistant_header_tokens = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: Some(gbnf_literal("a")),
                max_tokens: NonZeroU32::MIN,
                raw_prompt: QWEN3_ASSISTANT_HEADER_WITHOUT_THINKING.to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted")
        .summary()
        .expect("the generation must finish with a summary")
        .usage
        .prompt;

    assert_eq!(
        prompt_tokens_of_the_weather_question(true).await
            - prompt_tokens_of_the_weather_question(false).await,
        assistant_header_tokens
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
