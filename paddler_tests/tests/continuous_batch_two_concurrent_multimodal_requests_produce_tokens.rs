#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler::conversation_history::ConversationHistory;
use paddler::conversation_message::ConversationMessage;
use paddler::conversation_message_content::ConversationMessageContent;
use paddler::conversation_message_content_part::ConversationMessageContentPart;
use paddler::generated_token_result::GeneratedTokenResult;
use paddler::image_url::ImageUrl;
use paddler::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;
use paddler_tests::token_result_with_producer::TokenResultWithProducer;

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

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_two_concurrent_multimodal_requests_produce_tokens() -> Result<()> {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(4)], true).await?;

    let image_data_uri = load_test_image_data_uri()?;

    let params_a = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: build_multimodal_conversation(&image_data_uri),
        enable_thinking: false,
        grammar: None,
        max_tokens: 32,
        parse_tool_calls: false,
        tools: vec![],
    };
    let params_b = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: build_multimodal_conversation(&image_data_uri),
        enable_thinking: false,
        grammar: None,
        max_tokens: 32,
        parse_tool_calls: false,
        tools: vec![],
    };
    let (collected_a, collected_b) = tokio::join!(
        cluster.continue_from_conversation_history(&params_a),
        cluster.continue_from_conversation_history(&params_b),
    );

    let collected_a = collected_a?;
    let collected_b = collected_b?;

    for collected in [&collected_a, &collected_b] {
        let token_count = collected
            .token_results
            .iter()
            .filter(|result| result.token_result.is_token())
            .count();

        assert!(
            token_count > 0,
            "concurrent multimodal request should produce at least one token"
        );
        assert!(
            !collected
                .token_results
                .iter()
                .any(|result| matches!(result.token_result, GeneratedTokenResult::SamplerError(_))),
            "concurrent multimodal request must not surface a SamplerError"
        );
        assert!(matches!(
            collected.token_results.last(),
            Some(TokenResultWithProducer {
                token_result: GeneratedTokenResult::Done(_),
                ..
            })
        ));
    }

    cluster.shutdown().await?;

    Ok(())
}
