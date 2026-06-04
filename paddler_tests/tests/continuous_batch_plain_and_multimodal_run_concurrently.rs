#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_plain_and_multimodal_run_concurrently() -> Result<()> {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(4)], true).await?;

    let image_data_uri = load_test_image_data_uri()?;

    let multimodal_conversation = ConversationHistory::new(vec![
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
                        url: image_data_uri,
                    },
                },
                ConversationMessageContentPart::Text {
                    text: "Describe what you see in this image.".to_owned(),
                },
            ]),
            role: "user".to_owned(),
        },
    ]);

    let plain_params = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 64,
        raw_prompt: "Write a long poem about the sea.".to_owned(),
    };
    let multimodal_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: multimodal_conversation,
        enable_thinking: false,
        grammar: None,
        max_tokens: 32,
        parse_tool_calls: false,
        tools: vec![],
    };
    let (plain_collected, multimodal_collected) = tokio::join!(
        cluster.continue_from_raw_prompt(&plain_params),
        cluster.continue_from_conversation_history(&multimodal_params),
    );

    let plain_collected = plain_collected?;
    let multimodal_collected = multimodal_collected?;

    for (label, collected) in [
        ("plain", &plain_collected),
        ("multimodal", &multimodal_collected),
    ] {
        let token_count = collected
            .token_results
            .iter()
            .filter(|result| result.token_result.is_token())
            .count();

        assert!(
            token_count > 0,
            "concurrent {label} request should produce tokens"
        );
        assert!(
            !collected
                .token_results
                .iter()
                .any(|result| matches!(result.token_result, GeneratedTokenResult::SamplerError(_))),
            "concurrent {label} request must not surface a SamplerError"
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
