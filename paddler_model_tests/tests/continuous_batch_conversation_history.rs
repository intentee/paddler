#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_model_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_model_tests::log_generated_response::log_generated_response;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use tokio::sync::mpsc;

#[actix_web::test]
async fn test_concurrent_conversation_history_requests() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_string(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: None,
        slots: 2,
    })
    .await?;

    let (tx_a, rx_a) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx_a) = mpsc::unbounded_channel::<()>();
    let (tx_b, rx_b) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx_b) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(
            ContinuousBatchSchedulerCommand::ContinueFromConversationHistory(
                ContinueFromConversationHistoryRequest {
                    generated_tokens_tx: tx_a,
                    generate_tokens_stop_rx: stop_rx_a,
                    params: ContinueFromConversationHistoryParams {
                        add_generation_prompt: true,
                        conversation_history: ConversationHistory::new(vec![ConversationMessage {
                            content: ConversationMessageContent::Text("What is 2+2?".to_string()),
                            role: "user".to_string(),
                        }]),
                        enable_thinking: false,
                        grammar: None,
                        max_tokens: 20,
                        tools: vec![],
                    },
                },
            ),
        )
        .map_err(|err| anyhow::anyhow!("Failed to send first conversation: {err}"))?;

    managed_model
        .handle()
        .command_tx
        .send(
            ContinuousBatchSchedulerCommand::ContinueFromConversationHistory(
                ContinueFromConversationHistoryRequest {
                    generated_tokens_tx: tx_b,
                    generate_tokens_stop_rx: stop_rx_b,
                    params: ContinueFromConversationHistoryParams {
                        add_generation_prompt: true,
                        conversation_history: ConversationHistory::new(vec![ConversationMessage {
                            content: ConversationMessageContent::Text("Name a color".to_string()),
                            role: "user".to_string(),
                        }]),
                        enable_thinking: false,
                        grammar: None,
                        max_tokens: 20,
                        tools: vec![],
                    },
                },
            ),
        )
        .map_err(|err| anyhow::anyhow!("Failed to send second conversation: {err}"))?;

    let (results_a, results_b) = tokio::join!(
        collect_generated_tokens(rx_a),
        collect_generated_tokens(rx_b),
    );

    let results_a = results_a?;
    let results_b = results_b?;

    eprintln!("--- Conversation A ---");
    log_generated_response(&results_a);
    eprintln!("--- Conversation B ---");
    log_generated_response(&results_b);

    let tokens_a = results_a
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();
    let tokens_b = results_b
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(tokens_a > 0, "First conversation should produce tokens");
    assert!(tokens_b > 0, "Second conversation should produce tokens");

    assert!(
        matches!(results_a.last(), Some(GeneratedTokenResult::Done)),
        "First conversation should end with Done"
    );
    assert!(
        matches!(results_b.last(), Some(GeneratedTokenResult::Done)),
        "Second conversation should end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
