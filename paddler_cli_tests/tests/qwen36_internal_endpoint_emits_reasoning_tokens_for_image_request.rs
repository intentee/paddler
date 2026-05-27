#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_cli_tests::inference_http_client::InferenceHttpClient;
use paddler_cli_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_cli_tests::start_in_process_cluster_with_qwen3_6_and_mmproj::start_in_process_cluster_with_qwen3_6_and_mmproj;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::image_url::ImageUrl;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen36_internal_endpoint_emits_reasoning_tokens_for_image_request() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3_6_and_mmproj(AgentConfig::single(1)).await?;

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
        "Qwen 3.6: expected at least one reasoning token from a `<think>` block when an image is attached (got {reasoning_count})"
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
