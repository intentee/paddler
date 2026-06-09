#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_gemma_4_and_mmproj::start_cluster_with_gemma_4_and_mmproj;

#[tokio::test(flavor = "multi_thread")]
async fn gemma4_internal_endpoint_emits_reasoning_tokens_for_image_request() -> Result<()> {
    let cluster = start_cluster_with_gemma_4_and_mmproj(vec![AgentConfig::single(1)]).await?;

    let image_data_uri = load_test_image_data_uri()?;

    let conversation_history = ConversationHistory::new(vec![ConversationMessage {
        content: ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: image_data_uri,
                },
            },
            ConversationMessageContentPart::Text {
                text: "What animals do you see in this image? Think step by step.".to_owned(),
            },
        ]),
        role: "user".to_owned(),
    }]);

    let collected = cluster
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: true,
            grammar: None,
            max_tokens: 200,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let reasoning_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result.token_result, GeneratedTokenResult::ReasoningToken(_)))
        .count();

    assert!(
        reasoning_count > 0,
        "Gemma 4: expected at least one reasoning token from a `<|channel>thought` block when an image is attached (got {reasoning_count})"
    );

    let last = collected
        .token_results
        .last()
        .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.reasoning_tokens > 0);
    assert!(summary.usage.input_image_tokens > 0);

    cluster.shutdown().await?;

    Ok(())
}
