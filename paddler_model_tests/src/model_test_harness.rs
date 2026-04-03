use anyhow::Result;
use paddler::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use tokio::sync::mpsc;

use crate::managed_model::ManagedModel;

pub struct ModelTestHarness<'model> {
    managed_model: &'model ManagedModel,
}

impl<'model> ModelTestHarness<'model> {
    #[must_use]
    pub const fn new(managed_model: &'model ManagedModel) -> Self {
        Self { managed_model }
    }

    pub async fn generate_from_conversation(
        &self,
        params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<Vec<GeneratedTokenResult>> {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();
        let (_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel::<()>();

        self.managed_model
            .handle()
            .command_tx
            .send(
                ContinuousBatchSchedulerCommand::ContinueFromConversationHistory(
                    ContinueFromConversationHistoryRequest {
                        generated_tokens_tx,
                        generate_tokens_stop_rx,
                        params,
                    },
                ),
            )
            .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

        collect_generated_tokens(generated_tokens_rx).await
    }

    pub async fn generate_from_raw_prompt(
        &self,
        params: ContinueFromRawPromptParams,
    ) -> Result<Vec<GeneratedTokenResult>> {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();
        let (_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel::<()>();

        self.managed_model
            .handle()
            .command_tx
            .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
                ContinueFromRawPromptRequest {
                    generated_tokens_tx,
                    generate_tokens_stop_rx,
                    params,
                },
            ))
            .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

        collect_generated_tokens(generated_tokens_rx).await
    }
}

pub async fn collect_generated_tokens(
    mut generated_tokens_rx: mpsc::UnboundedReceiver<GeneratedTokenResult>,
) -> Result<Vec<GeneratedTokenResult>> {
    let mut results = Vec::new();

    while let Some(generated_token) = generated_tokens_rx.recv().await {
        let is_done = matches!(generated_token, GeneratedTokenResult::Done);

        results.push(generated_token);

        if is_done {
            break;
        }
    }

    Ok(results)
}
