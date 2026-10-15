#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::nomic_embed_desired_state_with_chat_template_override::nomic_embed_desired_state_with_chat_template_override;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_cannot_parse_tool_calls_for_a_model_without_a_chat_template() {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(
            nomic_embed_desired_state_with_chat_template_override(ChatTemplate {
                content: "{{ messages[0].content }}".to_owned(),
            }),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Hello".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(4).unwrap(),
                parse_tool_calls: true,
                tools: vec![get_weather_tool()],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(
        collected.into_token_results(),
        vec![GeneratedTokenResult::ChatTemplateError(
            "test-agent-0: failed to build this model's tool call parser: model has no chat template"
                .to_owned()
        )]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
