#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;

const MAX_TOKENS_ENDING_ON_A_SINGLE_NEWLINE_INSIDE_THE_REASONING: u32 = 336;

#[tokio::test(flavor = "multi_thread")]
async fn qwen35_thinking_cut_by_max_tokens_reports_every_generated_token_as_reasoning() {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(1)], false)
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![
                    ConversationMessage::Message {
                        content: ConversationMessageContent::Text("What is 2+2?".to_owned()),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: true,
                grammar: None,
                max_tokens: NonZeroU32::new(
                    MAX_TOKENS_ENDING_ON_A_SINGLE_NEWLINE_INSIDE_THE_REASONING,
                )
                .unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    let generated_token_count = collected
        .token_results
        .iter()
        .filter(|TokenResultWithProducer { token_result, .. }| token_result.is_token())
        .count();
    let reasoning_token_count = collected
        .token_results
        .iter()
        .filter(|TokenResultWithProducer { token_result, .. }| {
            matches!(token_result, GeneratedTokenResult::ReasoningToken(_))
        })
        .count();

    assert_eq!(reasoning_token_count, generated_token_count);

    let GenerationSummary { finish, usage } = collected
        .summary()
        .expect("the generation must finish with a summary");

    assert_eq!(finish, GenerationFinish::MaxTokens);
    assert_eq!(
        usage.reasoning_tokens,
        u64::from(MAX_TOKENS_ENDING_ON_A_SINGLE_NEWLINE_INSIDE_THE_REASONING)
    );
    assert_eq!(usage.content_tokens, 0);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
