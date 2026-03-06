#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
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
async fn test_qwen35_thinking_multi_turn_stops_cleanly() -> Result<()> {
    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters {
            min_p: 0.0,
            penalty_repeat: 1.1,
            top_k: 20,
            top_p: 0.95,
            ..InferenceParameters::default()
        },
        model: HuggingFaceModelReference {
            filename: "Qwen3.5-0.8B-Q6_K.gguf".to_string(),
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
                "You are a helpful assistant. Give engaging, short, precise answers. Be friendly, supportive, use emojis.".to_string(),
            ),
            role: "user".to_string(),
        },
        ConversationMessage {
            content: ConversationMessageContent::Text(
                "Hello! How can I help you today?".to_string(),
            ),
            role: "assistant".to_string(),
        },
        ConversationMessage {
            content: ConversationMessageContent::Text("hi".to_string()),
            role: "user".to_string(),
        },
    ]);

    let results = harness
        .generate_from_conversation(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: true,
            max_tokens: 1000,
            tools: vec![],
        })
        .await?;

    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    let full_response: String = results
        .iter()
        .filter_map(|result| match result {
            GeneratedTokenResult::Token(token) => Some(token.as_str()),
            _ => None,
        })
        .collect();

    eprintln!("Generated {token_count} tokens");
    eprintln!("Full response:\n{full_response}");

    assert!(
        token_count > 0,
        "Expected to receive at least one token from Qwen3.5 in thinking mode"
    );
    assert!(
        token_count < 1000,
        "Expected generation to stop before max_tokens (EOG token should be caught), got {token_count} tokens"
    );
    assert!(
        full_response.contains("</think>"),
        "Expected response to contain </think> tag in thinking mode, got:\n{full_response}"
    );
    assert!(
        matches!(results.last(), Some(GeneratedTokenResult::Done)),
        "Expected generation to end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
