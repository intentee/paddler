use std::sync::Arc;
use std::sync::mpsc::Sender;

use llama_cpp_bindings::llama_backend::LlamaBackend;
use tokio_util::task::TaskTracker;

use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::agent_request::AgentRequest;
use crate::continuous_batch_preparation_request::ContinuousBatchPreparationRequest;
use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::embedding_batch_preparer::EmbeddingBatchPreparer;
use crate::forward_scheduler_command::forward_scheduler_command;
use crate::generation_request_preparer::GenerationRequestPreparer;
use crate::generation_request_rejection::GenerationRequestRejection;
use crate::prepared_generation_request::PreparedGenerationRequest;

pub struct ContinuousBatchRequestPreparer {
    pub agent_name: Option<String>,
    pub embedding_batch_preparer: EmbeddingBatchPreparer,
    pub generation_request_preparer: GenerationRequestPreparer,
    pub llama_backend: Arc<LlamaBackend>,
    pub preparation_tasks: TaskTracker,
    pub scheduler_command_tx: Sender<ContinuousBatchSchedulerCommand>,
}

impl ContinuousBatchRequestPreparer {
    pub fn prepare(self: &Arc<Self>, request: ContinuousBatchPreparationRequest) {
        let preparer = self.clone();

        self.preparation_tasks
            .spawn_blocking(move || match request {
                ContinuousBatchPreparationRequest::ContinueFromConversationHistory(request) => {
                    preparer.accept_generation(
                        request,
                        GenerationRequestPreparer::prepare_conversation_history,
                    );
                }
                ContinuousBatchPreparationRequest::ContinueFromRawPrompt(request) => {
                    preparer
                        .accept_generation(request, GenerationRequestPreparer::prepare_raw_prompt);
                }
                ContinuousBatchPreparationRequest::GenerateEmbeddingBatch(request) => {
                    preparer.accept_embedding_batch(request);
                }
            });
    }

    pub async fn shut_down_scheduler_after_pending_preparations(&self) {
        self.preparation_tasks.close();
        self.preparation_tasks.wait().await;
        self.forward(ContinuousBatchSchedulerCommand::Shutdown);
    }

    fn forward(&self, command: ContinuousBatchSchedulerCommand) {
        forward_scheduler_command(
            &self.scheduler_command_tx,
            self.agent_name.as_deref(),
            command,
        );
    }

    fn accept_generation<TParams>(
        &self,
        request: AgentRequest<TParams, GeneratedTokenResult>,
        prepare: fn(
            &GenerationRequestPreparer,
            AgentRequest<TParams, GeneratedTokenResult>,
        ) -> Result<PreparedGenerationRequest, GenerationRequestRejection>,
    ) {
        let generated_tokens_tx = request.response_tx.clone();

        match prepare(&self.generation_request_preparer, request) {
            Ok(prepared) => self.forward(ContinuousBatchSchedulerCommand::Generate(Box::new(
                prepared,
            ))),
            Err(rejection) => rejection.report(self.agent_name.as_deref(), &generated_tokens_tx),
        }
    }

    fn accept_embedding_batch(
        &self,
        request: AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>,
    ) {
        let generated_embedding_tx = request.response_tx.clone();

        match self.embedding_batch_preparer.prepare(request) {
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
