#![cfg(feature = "tests_that_use_llms")]

use std::fs;
use std::future::Future;

use anyhow::Context as _;
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::cluster::Cluster;
use paddler_tests::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;

fn load_fixture_as_data_uri(fixture_name: &str, mime_type: &str) -> Result<String> {
    let fixture_path = format!("{}/../fixtures/{fixture_name}", env!("CARGO_MANIFEST_DIR"));
    let bytes = fs::read(&fixture_path)
        .with_context(|| format!("failed to read test fixture {fixture_path}"))?;
    let encoded = BASE64_STANDARD.encode(&bytes);

    Ok(format!("data:{mime_type};base64,{encoded}"))
}

fn drive_normal_image_fixture(
    cluster: &Cluster,
    fixture_name: &str,
    mime_type: &str,
) -> Result<impl Future<Output = Result<()>> + Send + use<>> {
    let image_data_uri = load_fixture_as_data_uri(fixture_name, mime_type)?;
    let fixture_name = fixture_name.to_owned();

    let generation =
        cluster.continue_from_conversation_history(&ContinueFromConversationHistoryParams {
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
            enable_thinking: false,
            grammar: None,
            max_tokens: 20,
            parse_tool_calls: false,
            tools: vec![],
        });

    Ok(async move {
        let collected = generation.await?;

        let saw_token = collected
            .token_results
            .iter()
            .any(|result| result.token_result.is_token());

        assert!(
            saw_token,
            "fixture {fixture_name} should produce at least one content/reasoning/tool-call token with adequate n_batch; got {:?}",
            collected
                .token_results
                .iter()
                .map(|result| &result.token_result)
                .collect::<Vec<_>>(),
        );

        Ok(())
    })
}

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_completes_generation_with_adequate_n_batch() -> Result<()> {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(1)]).await?;

    drive_normal_image_fixture(&cluster, "sarnow.jpeg", "image/jpeg")?.await?;
    drive_normal_image_fixture(&cluster, "llamas.webp", "image/webp")?.await?;

    cluster.shutdown().await?;

    Ok(())
}
