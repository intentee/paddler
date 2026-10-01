pub mod advance_generating_phase;
pub mod advance_outcome;
pub mod assemble_batch_phase;
pub mod batch_pass;
pub mod classified_token;
pub mod commit_phase;
pub mod completion_check_outcome;
pub mod completion_check_phase;
pub mod contributions;
pub mod decode_failure_phase;
pub mod emit_token_outcome;
pub mod emit_token_phase;
pub mod generating_contribution;
pub mod ingesting_contribution;
pub mod sequence_ordered_insertion_index;

use std::collections::VecDeque;
use std::mem::transmute;
use std::slice;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;

use llama_cpp_bindings::SampledTokenClassifier;
use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::DecodeError;
use llama_cpp_bindings::ingest_outcome::IngestOutcome;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;
use log::debug;
use log::error;
use log::info;
use tokio_util::sync::CancellationToken;

use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;

use self::advance_generating_phase::AdvanceGeneratingPhase;
use self::assemble_batch_phase::AssembleBatchPhase;
use self::batch_pass::BatchPass;
use self::commit_phase::CommitPhase;
use self::decode_failure_phase::DecodeFailurePhase;
use self::sequence_ordered_insertion_index::sequence_ordered_insertion_index;
use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_embedding_processor::ContinuousBatchEmbeddingProcessor;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_request_state::ContinuousBatchRequestState;
use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use crate::generation_request_rejection::GenerationRequestRejection;
use crate::multimodal_ingestion_progress::MultimodalIngestionProgress;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use crate::prepared_generation_request::PreparedGenerationRequest;
use crate::prepared_prompt::PreparedPrompt;
use crate::receives_stop_request::ReceivesStopRequest as _;
use crate::send_result_or_warn::send_result_or_warn;
use crate::sequence_id_guard::SequenceIdGuard;
use crate::sequence_id_pool::SequenceIdPool;
use crate::token_classification::TokenClassification;

pub struct ContinuousBatchScheduler {
    active_requests: Vec<ContinuousBatchActiveRequest>,
    agent_shutdown: CancellationToken,
    batch: LlamaBatch<'static>,
    candidates: LlamaTokenDataArray,
    command_rx: Receiver<ContinuousBatchSchedulerCommand>,
    ingest_outcomes: Vec<IngestOutcome>,
    llama_context: LlamaContext<'static>,
    pending_embedding_requests: VecDeque<PreparedEmbeddingBatchRequest>,
    scheduler_context: ContinuousBatchSchedulerContext,
    sequence_id_pool: SequenceIdPool,
    shutdown_requested: bool,
}

impl ContinuousBatchScheduler {
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "required for FFI lifetime extension with llama.cpp"
    )]
    pub fn new(
        ContinuousBatchSchedulerParams {
            agent_shutdown,
            batch,
            command_rx,
            llama_context,
            scheduler_context,
        }: ContinuousBatchSchedulerParams,
    ) -> Self {
        let llama_context =
            unsafe { transmute::<LlamaContext<'_>, LlamaContext<'static>>(llama_context) };
        let sequence_id_pool = SequenceIdPool::new(scheduler_context.desired_slots_total);

        Self {
            active_requests: Vec::new(),
            agent_shutdown,
            batch,
            candidates: LlamaTokenDataArray::new(Vec::new(), false),
            command_rx,
            ingest_outcomes: Vec::new(),
            llama_context,
            pending_embedding_requests: VecDeque::new(),
            scheduler_context,
            sequence_id_pool,
            shutdown_requested: false,
        }
    }

    pub fn run(&mut self) {
        info!(
            "{:?}: continuous batch scheduler started",
            self.scheduler_context.agent_name
        );

        while !self.agent_shutdown.is_cancelled() {
            self.check_stop_signals();
            self.remove_completed_requests();
            self.accept_new_commands();

            let has_active_requests = self.has_active_requests();

            self.try_process_embedding_request(has_active_requests);

            if has_active_requests {
                if let Err(batch_add_error) = self.execute_one_iteration() {
                    self.fail_unfinished_requests(GenerationRequestRejection::BatchAssemblyFailed(
                        batch_add_error,
                    ));
                }
            } else if self.pending_embedding_requests.is_empty() {
                if self.shutdown_requested {
                    break;
                }

                self.wait_for_next_command();
            }
        }

        while !self.active_requests.is_empty() {
            self.cleanup_completed_request(0);
        }

        self.llama_context.synchronize();
        self.llama_context.detach_threadpool();

        info!(
            "{:?}: continuous batch scheduler stopped",
            self.scheduler_context.agent_name
        );
    }

    fn wait_for_next_command(&mut self) {
        if let Ok(command) = self.command_rx.recv() {
            self.process_command(command);
        } else {
            info!(
                "{:?}: command channel closed, shutting down scheduler",
                self.scheduler_context.agent_name
            );
            self.shutdown_requested = true;
        }
    }

    fn accept_new_commands(&mut self) {
        loop {
            match self.command_rx.try_recv() {
                Ok(command) => self.process_command(command),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.shutdown_requested = true;

                    break;
                }
            }
        }
    }

    fn process_command(&mut self, command: ContinuousBatchSchedulerCommand) {
        match command {
            ContinuousBatchSchedulerCommand::Generate(request) => {
                self.accept_generation_request(*request);
            }
            ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(request) => {
                self.pending_embedding_requests.push_back(request);
            }
            ContinuousBatchSchedulerCommand::Shutdown => {
                self.shutdown_requested = true;
            }
        }
    }

    #[expect(
        unsafe_code,
        reason = "the SchedulerContext owns the LlamaModel for the lifetime of the active_requests vec — same pattern as LlamaContext<'static> above"
    )]
    fn build_token_classifier_for_active_request(
        &self,
        TokenClassification {
            bare_json_tool_calls,
            streaming_markers,
        }: TokenClassification,
    ) -> SampledTokenClassifier<'static> {
        let classifier = SampledTokenClassifier::new(
            &self.scheduler_context.model,
            streaming_markers,
            bare_json_tool_calls,
        );

        unsafe {
            transmute::<SampledTokenClassifier<'_>, SampledTokenClassifier<'static>>(classifier)
        }
    }

    fn accept_generation_request(&mut self, request: PreparedGenerationRequest) {
        let generated_tokens_tx = request.generated_tokens_tx.clone();

        match self.admit_generation_request(request) {
            Ok(active_request) => {
                debug!(
                    "{:?}: accepted generation request on sequence {}",
                    self.scheduler_context.agent_name,
                    active_request.sequence_id_guard.sequence_id()
                );

                let insertion_index = sequence_ordered_insertion_index(
                    self.active_requests
                        .iter()
                        .map(|request| request.sequence_id_guard.sequence_id()),
                    active_request.sequence_id_guard.sequence_id(),
                );

                self.active_requests.insert(insertion_index, active_request);
            }
            Err(rejection) => {
                rejection.report(
                    self.scheduler_context.agent_name.as_deref(),
                    &generated_tokens_tx,
                );
            }
        }
    }

    fn admit_generation_request(
        &mut self,
        PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens,
            prompt,
            slot_guard,
            token_classification,
            token_sampling,
            tool_call_handling,
        }: PreparedGenerationRequest,
    ) -> Result<ContinuousBatchActiveRequest, GenerationRequestRejection> {
        let sequence_id_guard = SequenceIdGuard::acquire(&self.sequence_id_pool)
            .ok_or(GenerationRequestRejection::NoSequenceSlotAvailable)?;
        let mut token_classifier =
            self.build_token_classifier_for_active_request(token_classification);

        self.clear_kv_cache_for_sequence(sequence_id_guard.sequence_id())?;

        let mut state = ContinuousBatchRequestState {
            current_token_position: 0,
            last_outcome_section: token_classifier.current_section(),
            max_tokens,
            phase: ContinuousBatchRequestPhase::IngestingText,
            prompt_tokens: Vec::new(),
            prompt_tokens_ingested: 0,
            sampled_tokens: 0,
        };

        match prompt {
            PreparedPrompt::TextTokens(prompt_tokens) => {
                token_classifier.record_prompt_tokens(prompt_tokens.len() as u64);
                token_classifier.ingest_prompt_tokens(&prompt_tokens);
                state.prompt_tokens = prompt_tokens;
            }
            PreparedPrompt::Multimodal(ingestion) => {
                state.phase = ContinuousBatchRequestPhase::IngestingMultimodal(ingestion);
            }
        }

        state.last_outcome_section = token_classifier.current_section();

        Ok(ContinuousBatchActiveRequest {
            state,
            token_classifier,
            token_sampling,
            generated_tokens_tx,
            generate_tokens_stop_rx,
            sequence_id_guard,
            slot_guard,
            tool_call_handling,
        })
    }

    fn ingest_next_multimodal_chunks(&mut self) {
        for active_request in &mut self.active_requests {
            let ContinuousBatchRequestPhase::IngestingMultimodal(ingestion) =
                &mut active_request.state.phase
            else {
                continue;
            };

            match ingestion.ingest_next_chunk(
                &mut active_request.token_classifier,
                &self.llama_context,
                active_request.sequence_id_guard.sequence_id(),
                active_request.state.current_token_position,
            ) {
                Ok(MultimodalIngestionProgress::ChunksRemain { next_position }) => {
                    active_request.state.current_token_position = next_position;
                }
                Ok(MultimodalIngestionProgress::PromptIngested { next_position }) => {
                    active_request
                        .state
                        .begin_generating_after_multimodal_prompt(
                            next_position,
                            active_request.token_classifier.current_section(),
                        );
                    self.llama_context.mark_logits_initialized(-1);

                    AdvanceGeneratingPhase {
                        candidates: &mut self.candidates,
                        ingest_outcomes: &mut self.ingest_outcomes,
                        llama_context: &self.llama_context,
                        scheduler_context: &self.scheduler_context,
                    }
                    .run(slice::from_mut(active_request));
                }
                Err(ingestion_error) => {
                    active_request.complete_with_outcome(
                        GenerationRequestRejection::MultimodalIngestionFailed(ingestion_error)
                            .into_generated_token_result(
                                self.scheduler_context.agent_name.as_deref(),
                            ),
                    );
                }
            }
        }
    }

    fn check_stop_signals(&mut self) {
        for active_request in &mut self.active_requests {
            if matches!(
                active_request.state.phase,
                ContinuousBatchRequestPhase::Completed(_)
            ) {
                continue;
            }

            if active_request.generate_tokens_stop_rx.is_stop_requested() {
                let summary = GenerationSummary {
                    finish: GenerationFinish::StopRequested,
                    usage: *active_request.token_classifier.usage(),
                };

                active_request.complete_with_outcome(GeneratedTokenResult::Done(summary));
            }
        }
    }

    fn try_process_embedding_request(&mut self, has_active_requests: bool) {
        let Some(request) = self.pending_embedding_requests.pop_front() else {
            return;
        };

        if has_active_requests {
            send_result_or_warn(
                self.scheduler_context.agent_name.as_deref(),
                &request.generated_embedding_tx,
                EmbeddingResult::EmbeddingRejectedDueToActiveTokenGeneration,
            );

            return;
        }

        let generated_embedding_tx = request.generated_embedding_tx.clone();
        let mut processor = ContinuousBatchEmbeddingProcessor::new(
            &mut self.batch,
            &mut self.llama_context,
            &self.scheduler_context,
        );

        if let Err(rejection) = processor.process_embedding_batch(request) {
            rejection.report(
                self.scheduler_context.agent_name.as_deref(),
                &generated_embedding_tx,
            );
        }
    }

    fn has_active_requests(&self) -> bool {
        self.active_requests.iter().any(|request| {
            !matches!(
                request.state.phase,
                ContinuousBatchRequestPhase::Completed(_)
            )
        })
    }

    fn fail_unfinished_requests(&mut self, rejection: GenerationRequestRejection) {
        let outcome =
            rejection.into_generated_token_result(self.scheduler_context.agent_name.as_deref());

        for active_request in &mut self.active_requests {
            active_request.complete_with_outcome(outcome.clone());
        }
    }

    fn execute_one_iteration(&mut self) -> Result<(), BatchAddError> {
        self.advance_generating_requests();
        self.ingest_next_multimodal_chunks();

        let assemble_phase = AssembleBatchPhase {
            n_batch: self
                .scheduler_context
                .inference_parameters
                .n_batch
                .tokens_usize(),
        };

        let mut pass = BatchPass::new(&mut self.batch);

        assemble_phase.run(&mut pass, &mut self.active_requests)?;

        if pass.is_empty() {
            return Ok(());
        }

        debug!(
            "{:?}: decoding batch with {} tokens for {} active requests",
            self.scheduler_context.agent_name,
            pass.batch.n_tokens(),
            self.active_requests.len()
        );

        match self.llama_context.decode(pass.batch) {
            Ok(()) => {
                CommitPhase {
                    requests: &mut self.active_requests,
                }
                .run(pass);
            }
            Err(DecodeError::Aborted) => {}
            Err(decode_error) => {
                let description = format!(
                    "{:?}: llama.cpp rejected the batch: {decode_error}",
                    self.scheduler_context.agent_name
                );

                error!("{description}");
                DecodeFailurePhase {
                    requests: &mut self.active_requests,
                }
                .run(pass, &description);
            }
        }

        Ok(())
    }

    fn advance_generating_requests(&mut self) {
        AdvanceGeneratingPhase {
            candidates: &mut self.candidates,
            ingest_outcomes: &mut self.ingest_outcomes,
            llama_context: &self.llama_context,
            scheduler_context: &self.scheduler_context,
        }
        .run(&mut self.active_requests);
    }

    fn remove_completed_requests(&mut self) {
        let mut removal_index = 0;

        while removal_index < self.active_requests.len() {
            if matches!(
                self.active_requests[removal_index].state.phase,
                ContinuousBatchRequestPhase::Completed(_)
            ) {
                self.cleanup_completed_request(removal_index);
            } else {
                removal_index += 1;
            }
        }
    }

    fn clear_kv_cache_for_sequence(
        &mut self,
        sequence_id: i32,
    ) -> Result<(), GenerationRequestRejection> {
        let kv_cache_sequence_id = u32::try_from(sequence_id).map_err(|source| {
            GenerationRequestRejection::SequenceIdOutOfRange {
                sequence_id,
                source,
            }
        })?;

        self.llama_context
            .clear_kv_cache_seq(Some(kv_cache_sequence_id), None, None)
            .map_err(GenerationRequestRejection::KvCacheClearFailed)
    }

    fn cleanup_completed_request(&mut self, index: usize) {
        let removed_request = self.active_requests.remove(index);
        let sequence_id = removed_request.sequence_id_guard.sequence_id();
        let usage = *removed_request.token_classifier.usage();
        let terminal_delivery = removed_request.into_terminal_delivery();

        if let Err(rejection) = self.clear_kv_cache_for_sequence(sequence_id) {
            error!(
                "{:?}: {rejection}; the next request admitted to sequence {sequence_id} clears it again",
                self.scheduler_context.agent_name
            );
        }

        debug!(
            "{:?}: cleaned up sequence {sequence_id} ({} completion tokens generated)",
            self.scheduler_context.agent_name,
            usage.completion_tokens(),
        );

        terminal_delivery.deliver(self.scheduler_context.agent_name.as_deref());
    }
}
