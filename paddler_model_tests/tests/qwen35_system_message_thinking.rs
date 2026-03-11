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

#[actix_web::test]
async fn test_qwen35_system_and_user_messages_with_thinking() -> Result<()> {
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

    let conversation_history = ConversationHistory::new(vec![
        ConversationMessage {
            content: ConversationMessageContent::Text(
                "You are a focused web crawler assistant. Your only job is to decide which links \
                to follow to discover more relevant pages. Respond with JSON only."
                    .to_string(),
            ),
            role: "system".to_string(),
        },
        ConversationMessage {
            content: ConversationMessageContent::Text(
                "Goal: \"find all PDF reports\"\n\n\
                Page: https://example.com/reports\n\n\
                Followable links:\n\
                [0] [Navigation] \"Home\" → /home\n\
                [1] [PrimaryListing] \"Annual Report 2024\" → /reports/annual-2024.pdf\n\
                [2] [PrimaryListing] \"Q3 Financial Summary\" → /reports/q3-summary.pdf\n\
                [3] [Navigation] \"Next Page\" → /reports?page=2"
                    .to_string(),
            ),
            role: "user".to_string(),
        },
    ]);

    let results = harness
        .generate_from_conversation(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: true,
            max_tokens: 2000,
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
        "Expected to receive at least one token from Qwen3.5 with system+user messages and thinking"
    );
    assert!(
        matches!(results.last(), Some(GeneratedTokenResult::Done)),
        "Expected generation to end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
