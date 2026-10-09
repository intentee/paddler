#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

const CAPITAL_OF_FRANCE_MAX_TOKENS: NonZeroU32 = NonZeroU32::new(10).unwrap();

async fn capital_of_france(cluster: &Cluster) -> Result<()> {
    cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "The capital of France is".to_owned(),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: CAPITAL_OF_FRANCE_MAX_TOKENS,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await?
        .summary()?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_swaps_between_inference_calls() {
    let mut cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();
    let swapped_template = ChatTemplate {
        content: "PREFIX:{{ messages[0].content }}".to_owned(),
    };

    capital_of_france(&cluster)
        .await
        .expect("the capital of France request must be answered");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &desired_state_with_chat_template_override(
                qwen3_desired_state(),
                swapped_template.clone(),
            ),
        )
        .await
        .expect("the balancer must accept the desired state");
    cluster
        .wait_for_chat_template_override_in_use(&agent_id)
        .await
        .expect("the agent must use the chat template override");

    capital_of_france(&cluster)
        .await
        .expect("the capital of France request must be answered");

    assert_eq!(
        cluster
            .client_management
            .get_chat_template_override(CancellationToken::new(), &agent_id)
            .await
            .expect("the agent must report its chat template override"),
        Some(swapped_template)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
