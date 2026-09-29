#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use futures_util::StreamExt as _;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;
use tokio_util::sync::CancellationToken;

const FORCED_WORD: &str = "apple ";
const FORCED_WORD_REPETITIONS: usize = 1000;

#[tokio::test(flavor = "multi_thread")]
async fn generating_request_keeps_every_token_when_multimodal_request_is_admitted() -> Result<()> {
    let cluster = start_cluster_with_smolvlm2(AgentConfig::uniform(1, 2)).await?;
    let forced_text = FORCED_WORD.repeat(FORCED_WORD_REPETITIONS);

    let mut text_stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: Some(GrammarConstraint::Gbnf {
                    grammar: format!("root ::= \"{forced_text}\""),
                    root: "root".to_owned(),
                }),
                max_tokens: NonZeroU32::try_from(u32::try_from(forced_text.len())?)?,
                raw_prompt: "Repeat the word apple.".to_owned(),
            },
        )
        .await?;

    let Message::Response(ResponseEnvelope {
        response: Response::GeneratedToken(first_token_result),
        ..
    }) = text_stream
        .next()
        .await
        .context("the text request must stream a first token")??
    else {
        bail!("the text request must start by streaming a generated token");
    };

    let multimodal_request = cluster.continue_from_conversation_history(
        CancellationToken::new(),
        &ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: load_test_image_data_uri()?,
                        },
                    },
                    ConversationMessageContentPart::Text {
                        text: "Describe this image.".to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: NonZeroU32::new(4).unwrap(),
            parse_tool_calls: false,
            tools: vec![],
        },
    );

    let (remaining_text, multimodal_collected) =
        tokio::join!(collect_generated_tokens(text_stream), multimodal_request);

    multimodal_collected?;

    let streamed_text = format!(
        "{}{}",
        first_token_result.token_text().unwrap_or_default(),
        remaining_text?.text
    );

    assert_eq!(streamed_text, forced_text);

    cluster.shutdown().await?;

    Ok(())
}
