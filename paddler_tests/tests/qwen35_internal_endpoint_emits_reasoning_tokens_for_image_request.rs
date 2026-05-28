#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;
use paddler::conversation_history::ConversationHistory;
use paddler::conversation_message::ConversationMessage;
use paddler::conversation_message_content::ConversationMessageContent;
use paddler::conversation_message_content_part::ConversationMessageContentPart;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::image_url::ImageUrl;
use paddler::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen35_internal_endpoint_emits_reasoning_tokens_for_image_request() -> Result<()> {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(1)], true).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

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

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: true,
            grammar: None,
            max_tokens: 200,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let reasoning_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result.token_result, GeneratedTokenResult::ReasoningToken(_)))
        .count();

    assert!(
        reasoning_count > 0,
        "Qwen 3.5: expected at least one reasoning token from a `<think>` block when an image is attached (got {reasoning_count})"
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
