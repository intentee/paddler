#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;

use crate::qwen3_without_a_chat_template::Qwen3WithoutAChatTemplate;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_without_a_chat_template_refuses_conversations() {
    let qwen3_without_a_chat_template = Qwen3WithoutAChatTemplate::write();
    let cluster = start_single_agent_cluster_with_desired_state(
        qwen3_without_a_chat_template.desired_state(),
    )
    .await
    .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![
                    ConversationMessage::Message {
                        content: ConversationMessageContent::Text(
                            "Name a primary color.".to_owned(),
                        ),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the conversation must be accepted");

    assert_eq!(
        collected.into_token_results(),
        vec![GeneratedTokenResult::ChatTemplateError(format!(
            "test-agent-0: the model {} has no chat template, so it cannot continue a conversation",
            qwen3_without_a_chat_template.model_path().display()
        ))]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
