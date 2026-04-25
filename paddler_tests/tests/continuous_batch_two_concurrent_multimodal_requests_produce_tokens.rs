#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_in_process_cluster_with_qwen3_5::start_in_process_cluster_with_qwen3_5;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::image_url::ImageUrl;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

fn build_multimodal_conversation(image_data_uri: &str) -> ConversationHistory {
    ConversationHistory::new(vec![
        ConversationMessage {
            content: ConversationMessageContent::Text(
                "You are a helpful assistant. Give engaging, short, precise answers.".to_owned(),
            ),
            role: "system".to_owned(),
        },
        ConversationMessage {
            content: ConversationMessageContent::Text(
                "Hello! How can I help you today?".to_owned(),
            ),
            role: "assistant".to_owned(),
        },
        ConversationMessage {
            content: ConversationMessageContent::Parts(vec![
                ConversationMessageContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: image_data_uri.to_owned(),
                    },
                },
                ConversationMessageContentPart::Text {
                    text: "Describe what you see in this image.".to_owned(),
                },
            ]),
            role: "user".to_owned(),
        },
    ])
}

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_two_concurrent_multimodal_requests_produce_tokens() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3_5(4, true).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let image_data_uri = load_test_image_data_uri()?;

    let stream_a = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: build_multimodal_conversation(&image_data_uri),
            enable_thinking: false,
            grammar: None,
            max_tokens: 32,
            tools: vec![],
        })
        .await?;

    let stream_b = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: build_multimodal_conversation(&image_data_uri),
            enable_thinking: false,
            grammar: None,
            max_tokens: 32,
            tools: vec![],
        })
        .await?;

    let (collected_a, collected_b) = tokio::join!(
        collect_generated_tokens(stream_a),
        collect_generated_tokens(stream_b),
    );

    let collected_a = collected_a?;
    let collected_b = collected_b?;

    for collected in [&collected_a, &collected_b] {
        let token_count = collected
            .token_results
            .iter()
            .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
            .count();

        assert!(
            token_count > 0,
            "concurrent multimodal request should produce at least one token"
        );
        assert!(
            !collected
                .token_results
                .iter()
                .any(|result| matches!(result, GeneratedTokenResult::SamplerError(_))),
            "concurrent multimodal request must not surface a SamplerError"
        );
        assert!(matches!(
            collected.token_results.last(),
            Some(GeneratedTokenResult::Done)
        ));
    }

    cluster.shutdown().await?;

    Ok(())
}
