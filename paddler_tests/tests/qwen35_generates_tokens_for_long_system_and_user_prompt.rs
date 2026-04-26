#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3_5::start_in_process_cluster_with_qwen3_5;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

fn build_long_link_list() -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("[2] [Unknown] \"Example Organization\" → /".to_owned());
    lines.push("[4] [Unknown] \"Example Organization\" → /".to_owned());
    lines.push("[5] [Navigation] \"Back to: Home\" → /".to_owned());

    for index in 6..=20 {
        lines.push(format!(
            "[{index}] [Navigation] \"Section {index}\" → /section-{index}"
        ));
    }
    for index in 25..=34 {
        lines.push(format!(
            "[{index}] [Navigation] \"Menu Item {index}\" → /menu/{index}"
        ));
    }
    for index in 47..=58 {
        lines.push(format!(
            "[{index}] [PrimaryListing] \"Research Report {index}: Analysis of cultural participation patterns in region {index} during annual review\" → /reports/report-{index}"
        ));
    }
    for index in 59..=64 {
        lines.push(format!(
            "[{index}] [Navigation] \"{index} page of article list\" → /reports?p={index}"
        ));
    }
    for index in 65..=78 {
        lines.push(format!(
            "[{index}] [Navigation] \"Footer Link {index}\" → /footer/{index}"
        ));
    }
    lines.push("[88] [Unknown] \"Close search window\" → ".to_owned());

    lines.join("\n")
}

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen35_generates_tokens_for_long_system_and_user_prompt() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3_5(1, false).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let system_prompt = "You are a focused web crawler assistant. All elements on each page are collected automatically. Your only job is to decide which links to FOLLOW to discover more relevant pages.\n\nGiven a user's goal and the followable links extracted from a web page, decide which links are worth following to find more content matching the goal.\n\nRespond with JSON only:\n{\"follow\": [1, 3]}\n\nRules:\n- \"follow\": original indices of link elements worth following\n- Reject links that are clearly irrelevant to the goal\n- Prefer following PrimaryListing links on index/listing pages\n- Follow pagination links if more matching content is likely on subsequent pages\n- If no links are worth following, return {\"follow\": []}";

    let user_prompt = format!(
        "Goal: \"find all PDF reports\"\n\nPage: https://example.com/reports\n\nFollowable links:\n{}",
        build_long_link_list()
    );

    let conversation_history = ConversationHistory::new(vec![
        ConversationMessage {
            content: ConversationMessageContent::Text(system_prompt.to_owned()),
            role: "system".to_owned(),
        },
        ConversationMessage {
            content: ConversationMessageContent::Text(user_prompt),
            role: "user".to_owned(),
        },
    ]);

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: false,
            grammar: None,
            max_tokens: 512,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count > 0);
    assert!(matches!(
        collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    cluster.shutdown().await?;

    Ok(())
}
