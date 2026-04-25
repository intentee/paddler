#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_subprocess_cluster_with_smolvlm2::start_subprocess_cluster_with_smolvlm2;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::image_url::ImageUrl;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn agent_streams_tokens_from_image_data_uri() -> Result<()> {
    let cluster = start_subprocess_cluster_with_smolvlm2(4, 1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let image_data_uri = load_test_image_data_uri()?;

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
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
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 100,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let received_tokens = collected
        .token_results
        .iter()
        .any(|result| matches!(result, GeneratedTokenResult::Token(_)));

    assert!(received_tokens);

    cluster.shutdown().await?;

    Ok(())
}
