#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_conversation_with_function_tool_succeeds() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Say hello".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: true,
                grammar: None,
                max_tokens: NonZeroU32::new(50).unwrap(),
                parse_tool_calls: true,
                tools: vec![get_weather_tool()],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert!(
        !collected.token_results.is_empty(),
        "should receive a response when a function tool is provided"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
