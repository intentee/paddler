#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_arbiter::ContinuousBatchArbiter;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler::agent::model_metadata_holder::ModelMetadataHolder;
use paddler::agent_desired_state::AgentDesiredState;
use paddler::converts_to_applicable_state::ConvertsToApplicableState;
use paddler::slot_aggregated_status_manager::SlotAggregatedStatusManager;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use tokio::sync::mpsc;

/// After a scheduler shuts down with active requests, slots_processing_count
/// must be 0. If it leaks, drain_in_flight_requests hangs forever on the
/// next state change.
#[actix_web::test]
async fn test_slots_processing_count_zero_after_shutdown_with_active_request() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(1));

    let desired_state = AgentDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_string(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_string(),
            revision: "main".to_string(),
        }),
        multimodal_projection: AgentDesiredModel::None,
    };

    let applicable_state = desired_state
        .to_applicable_state(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .clone(),
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to convert to applicable state"))?;

    let model_path = applicable_state
        .model_path
        .ok_or_else(|| anyhow::anyhow!("Model path is required"))?;

    let arbiter = ContinuousBatchArbiter {
        agent_name: Some("slot_leak_test".to_owned()),
        chat_template_override: None,
        desired_slots_total: 1,
        inference_parameters: applicable_state.inference_parameters,
        multimodal_projection_path: applicable_state.multimodal_projection_path,
        model_metadata_holder: Arc::new(ModelMetadataHolder::new()),
        model_path: model_path.clone(),
        model_path_string: model_path.display().to_string(),
        slot_aggregated_status_manager: slot_aggregated_status_manager.clone(),
    };

    let mut handle = arbiter.spawn().await?;

    // Start generation
    let (gen_tx, mut gen_rx) = mpsc::unbounded_channel();
    let (_, gen_stop_rx) = mpsc::unbounded_channel::<()>();

    handle
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx: gen_tx,
                generate_tokens_stop_rx: gen_stop_rx,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 500,
                    raw_prompt: "Write a long essay".to_string(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

    // Wait for generation to start
    let first = gen_rx.recv().await;
    assert!(matches!(first, Some(GeneratedTokenResult::Token(_))));

    // Simulate client disconnect
    drop(gen_rx);

    // Shutdown scheduler while request is active
    handle.shutdown()?;

    // The critical assertion: after shutdown, all slots must be released
    let slots_processing = slot_aggregated_status_manager
        .slot_aggregated_status
        .slots_processing_count();

    assert_eq!(
        slots_processing, 0,
        "slots_processing_count must be 0 after shutdown, got {slots_processing}"
    );

    Ok(())
}
