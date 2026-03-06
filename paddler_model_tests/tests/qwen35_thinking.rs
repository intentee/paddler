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
async fn test_qwen35_thinking_mode_stops_cleanly() -> Result<()> {
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

    let conversation_history = ConversationHistory::new(vec![ConversationMessage {
        content: ConversationMessageContent::Text("What is 2+2?".to_string()),
        role: "user".to_string(),
    }]);

    let results = harness
        .generate_from_conversation(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: true,
            max_tokens: 2000,
            tools: vec![],
        })
        .await?;

    let thinking_token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::ThinkingToken(_)))
        .count();

    let response_token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    let total_token_count = thinking_token_count + response_token_count;

    assert!(
        thinking_token_count > 0,
        "Expected to receive at least one ThinkingToken from Qwen3.5 in thinking mode"
    );
    assert!(
        response_token_count > 0,
        "Expected to receive at least one response Token after thinking"
    );
    assert!(
        total_token_count < 2000,
        "Expected generation to stop before max_tokens (EOG token should be caught), got {total_token_count} tokens"
    );
    assert!(
        matches!(results.last(), Some(GeneratedTokenResult::Done)),
        "Expected generation to end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
