#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler_harness::log_generated_response::log_generated_response;
use paddler_harness::managed_model::ManagedModel;
use paddler_harness::managed_model::ManagedModelParams;
use paddler_harness::model_test_harness::ModelTestHarness;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

fn build_long_link_list() -> String {
    let mut lines = Vec::new();

    lines.push("[2] [Unknown] \"Example Organization\" → /".to_string());
    lines.push("[4] [Unknown] \"Example Organization\" → /".to_string());
    lines.push("[5] [Navigation] \"Back to: Home\" → /".to_string());

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

    lines.push("[88] [Unknown] \"Close search window\" → ".to_string());

    lines.join("\n")
}

#[actix_web::test]
async fn test_qwen35_long_prompt_with_system_message() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen3.5-0.8B-Q4_K_M.gguf".to_string(),
            repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: None,
    })
    .await?;

    let harness = ModelTestHarness::new(&managed_model);

    let system_prompt = "\
        You are a focused web crawler assistant. All elements on each page are collected \
        automatically. Your only job is to decide which links to FOLLOW to discover more \
        relevant pages.\n\n\
        Given a user's goal and the followable links extracted from a web page, decide \
        which links are worth following to find more content matching the goal.\n\n\
        Respond with JSON only:\n\
        {\"follow\": [1, 3]}\n\n\
        Rules:\n\
        - \"follow\": original indices of link elements worth following\n\
        - Reject links that are clearly irrelevant to the goal\n\
        - Prefer following PrimaryListing links on index/listing pages\n\
        - Follow pagination links if more matching content is likely on subsequent pages\n\
        - If no links are worth following, return {\"follow\": []}";

    let user_prompt = format!(
        "Goal: \"find all PDF reports\"\n\n\
        Page: https://example.com/reports\n\n\
        Followable links:\n{}",
        build_long_link_list()
    );

    let conversation_history = ConversationHistory::new(vec![
        ConversationMessage {
            content: ConversationMessageContent::Text(system_prompt.to_string()),
            role: "system".to_string(),
        },
        ConversationMessage {
            content: ConversationMessageContent::Text(user_prompt),
            role: "user".to_string(),
        },
    ]);

    let results = harness
        .generate_from_conversation(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: false,
            max_tokens: 512,
            tools: vec![],
        })
        .await?;

    log_generated_response(&results);

    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(
        token_count > 0,
        "Expected to receive at least one token from Qwen3.5 with a long system+user prompt ({} followable links)",
        build_long_link_list().lines().count()
    );
    assert!(
        matches!(results.last(), Some(GeneratedTokenResult::Done)),
        "Expected generation to end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
