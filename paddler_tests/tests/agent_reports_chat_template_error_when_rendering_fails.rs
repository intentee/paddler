#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_chat_template_error_when_rendering_fails() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: Some(BalancerDesiredState {
            chat_template_override: Some(ChatTemplate {
                content: "{{ raise_exception('conversations are not supported') }}".to_owned(),
            }),
            use_chat_template_override: true,
            ..qwen3_desired_state()
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;

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
                max_tokens: 4,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await?;

    assert!(matches!(
        collected
            .token_results
            .iter()
            .map(|token_result_with_producer| &token_result_with_producer.token_result)
            .collect::<Vec<&GeneratedTokenResult>>()
            .as_slice(),
        [GeneratedTokenResult::ChatTemplateError(_)]
    ));

    cluster.shutdown().await?;

    Ok(())
}
