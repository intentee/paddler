use std::sync::Arc;
use std::sync::mpsc::Sender;

use tokio::sync::mpsc;

use crate::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::continuous_batch_arbiter_command::ContinuousBatchArbiterCommand;
use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::embedding_batch_preparer::EmbeddingBatchPreparer;
use crate::forward_scheduler_command::forward_scheduler_command;
use crate::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::generation_request_preparer::GenerationRequestPreparer;
use crate::prepared_generation_request::PreparedGenerationRequest;

#[derive(Clone)]
pub struct ContinuousBatchRequestPreparer {
    pub agent_name: Option<String>,
    pub embedding_batch_preparer: Arc<EmbeddingBatchPreparer>,
    pub generation_request_preparer: Arc<GenerationRequestPreparer>,
    pub scheduler_command_tx: Sender<ContinuousBatchSchedulerCommand>,
}

impl ContinuousBatchRequestPreparer {
    pub async fn run(
        self,
        mut arbiter_command_rx: mpsc::UnboundedReceiver<ContinuousBatchArbiterCommand>,
    ) {
        while let Some(command) = arbiter_command_rx.recv().await {
            let preparer = self.clone();

            match command {
                ContinuousBatchArbiterCommand::ContinueFromConversationHistory(request) => {
                    tokio::task::spawn_blocking(move || {
                        preparer.accept_conversation_history(request);
                    });
                }
                ContinuousBatchArbiterCommand::ContinueFromRawPrompt(request) => {
                    tokio::task::spawn_blocking(move || preparer.accept_raw_prompt(request));
                }
                ContinuousBatchArbiterCommand::GenerateEmbeddingBatch(request) => {
                    tokio::task::spawn_blocking(move || preparer.accept_embedding_batch(request));
                }
                ContinuousBatchArbiterCommand::Shutdown => {
                    self.forward(ContinuousBatchSchedulerCommand::Shutdown);

                    return;
                }
            }
        }
    }

    fn forward(&self, command: ContinuousBatchSchedulerCommand) {
        forward_scheduler_command(
            &self.scheduler_command_tx,
            self.agent_name.as_deref(),
            command,
        );
    }

    fn forward_generation(&self, prepared: PreparedGenerationRequest) {
        self.forward(ContinuousBatchSchedulerCommand::Generate(Box::new(
            prepared,
        )));
    }

    fn accept_raw_prompt(&self, request: ContinueFromRawPromptRequest) {
        let generated_tokens_tx = request.generated_tokens_tx.clone();

        match self.generation_request_preparer.prepare_raw_prompt(request) {
            Ok(prepared) => self.forward_generation(prepared),
            Err(rejection) => rejection.report(self.agent_name.as_deref(), &generated_tokens_tx),
        }
    }

    fn accept_conversation_history(&self, request: ContinueFromConversationHistoryRequest) {
        let generated_tokens_tx = request.generated_tokens_tx.clone();

        match self
            .generation_request_preparer
            .prepare_conversation_history(request)
        {
            Ok(prepared) => self.forward_generation(prepared),
            Err(rejection) => rejection.report(self.agent_name.as_deref(), &generated_tokens_tx),
        }
    }

    fn accept_embedding_batch(&self, request: GenerateEmbeddingBatchRequest) {
        let generated_embedding_tx = request.generated_embedding_tx.clone();

        match self
            .embedding_batch_preparer
            .prepare(self.agent_name.as_deref(), request)
        {
            Ok(prepared) => {
                self.forward(ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(
                    prepared,
                ));
            }
            Err(rejection) => {
                rejection.report(self.agent_name.as_deref(), &generated_embedding_tx);
            }
        }
    }
}
