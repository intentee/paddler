#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_in_process_cluster_with_qwen2_5_vl::start_in_process_cluster_with_qwen2_5_vl;
use paddler_tests::token_result_with_producer::TokenResultWithProducer;
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
async fn qwen25vl_generates_tokens_from_image_input() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen2_5_vl(AgentConfig::single(1)).await?;

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
                text: "What do you see in this image?".to_owned(),
            },
        ]),
        role: "user".to_owned(),
    }]);

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: false,
            grammar: None,
            max_tokens: 200,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(token_count > 0);
    assert!(matches!(
        collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    cluster.shutdown().await?;

    Ok(())
}
