pub mod advance_generating_phase;
pub mod advance_outcome;
pub mod assemble_batch_phase;
pub mod batch_pass;
pub mod classified_token;
pub mod classify_token_phase;
pub mod commit_phase;
pub mod completion_check_outcome;
pub mod completion_check_phase;
pub mod contributions;
pub mod decode_batch_phase;
pub mod decode_outcome;
pub mod emit_token_outcome;
pub mod emit_token_phase;
pub mod generating_contribution;
pub mod ingesting_contribution;
pub mod sample_outcome;
pub mod sample_token_phase;
pub mod sequence_ordered_insertion_index;
pub mod tool_call_pass;

use std::collections::VecDeque;
use std::slice;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;

use anyhow::Result;
use llama_cpp_bindings::SampledTokenClassifier;
use llama_cpp_bindings::StreamingMarkers;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::SamplingError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::sampling::LlamaSampler;
use log::debug;
use log::error;
use log::info;
use log::warn;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use rand::Rng as _;
use rand::rngs::ThreadRng;
use tokio_util::sync::CancellationToken;

use self::advance_generating_phase::AdvanceGeneratingPhase;
use self::assemble_batch_phase::AssembleBatchPhase;
use self::batch_pass::BatchPass;
use self::decode_outcome::DecodeOutcome;
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
use crate::sampler_chain_factory::SamplerChainFactory;
use crate::sequence_id_guard::SequenceIdGuard;
use crate::sequence_id_pool::SequenceIdPool;

pub struct ContinuousBatchScheduler {
    active_requests: Vec<ContinuousBatchActiveRequest>,
    agent_shutdown: CancellationToken,
    batch: LlamaBatch<'static>,
    command_rx: Receiver<ContinuousBatchSchedulerCommand>,
    llama_context: LlamaContext<'static>,
    pending_embedding_requests: VecDeque<PreparedEmbeddingBatchRequest>,
    rng: ThreadRng,
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
        let llama_context = unsafe {
            std::mem::transmute::<LlamaContext<'_>, LlamaContext<'static>>(llama_context)
        };
        let sequence_id_pool = SequenceIdPool::new(scheduler_context.desired_slots_total);

        Self {
            active_requests: Vec::new(),
            agent_shutdown,
            batch,
            command_rx,
            llama_context,
            pending_embedding_requests: VecDeque::new(),
            rng: rand::rng(),
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
                if let Err(err) = self.execute_one_iteration() {
                    error!(
                        "{:?}: scheduler iteration failed: {err:#}",
                        self.scheduler_context.agent_name
                    );
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

    fn create_sampler_chain(&mut self) -> Result<LlamaSampler, SamplingError> {
        SamplerChainFactory {
            inference_parameters: &self.scheduler_context.inference_parameters,
            n_vocab: self.scheduler_context.n_vocab,
        }
        .create(self.rng.random::<u32>())
    }

    #[expect(
        unsafe_code,
        reason = "the SchedulerContext owns the LlamaModel for the lifetime of the active_requests vec — same pattern as LlamaContext<'static> above"
    )]
    fn build_token_classifier_for_active_request(
        &self,
        streaming_markers: Arc<StreamingMarkers>,
    ) -> SampledTokenClassifier<'static> {
        let classifier =
            SampledTokenClassifier::new(&self.scheduler_context.model, streaming_markers);

        unsafe {
            std::mem::transmute::<SampledTokenClassifier<'_>, SampledTokenClassifier<'static>>(
                classifier,
            )
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
            grammar_sampler,
            max_tokens,
            prompt,
            slot_guard,
            streaming_markers,
            tool_call_pipeline,
        }: PreparedGenerationRequest,
    ) -> Result<ContinuousBatchActiveRequest, GenerationRequestRejection> {
        let sequence_id_guard = SequenceIdGuard::acquire(&self.sequence_id_pool)
            .ok_or(GenerationRequestRejection::NoSequenceSlotAvailable)?;
        let grammar_sampler = grammar_sampler
            .map(|grammar_sampler| {
                grammar_sampler.into_llama_sampler(&self.scheduler_context.model)
            })
            .transpose()
            .map_err(GenerationRequestRejection::GrammarSamplerInitializationFailed)?;
        let chain = self
            .create_sampler_chain()
            .map_err(GenerationRequestRejection::SamplerChainCreationFailed)?;
        let token_classifier = self.build_token_classifier_for_active_request(streaming_markers);

        self.clear_kv_cache_for_sequence(sequence_id_guard.sequence_id());

        let mut active_request = ContinuousBatchActiveRequest {
            state: ContinuousBatchRequestState {
                current_token_position: 0,
                i_batch: None,
                last_outcome_section: token_classifier.current_section(),
                max_tokens,
                pending_sampled_token: None,
                phase: ContinuousBatchRequestPhase::IngestingText,
                prompt_tokens: Vec::new(),
                prompt_tokens_ingested: 0,
            },
            chain,
            token_classifier,
            grammar_sampler,
            generated_tokens_tx,
            generate_tokens_stop_rx,
            sequence_id_guard,
            slot_guard,
            tool_call_pipeline,
        };

        match prompt {
            PreparedPrompt::TextTokens(prompt_tokens) => {
                active_request
                    .token_classifier
                    .record_prompt_tokens(prompt_tokens.len() as u64);
                active_request
                    .token_classifier
                    .ingest_prompt_tokens(&prompt_tokens);
                active_request.state.prompt_tokens = prompt_tokens;
            }
            PreparedPrompt::Multimodal(multimodal_prompt) => {
                active_request.state.phase = ContinuousBatchRequestPhase::IngestingMultimodal(
                    multimodal_prompt
                        .into_ingestion(self.scheduler_context.sequence_context_size)?,
                );
            }
        }

        active_request.state.last_outcome_section =
            active_request.token_classifier.current_section();

        Ok(active_request)
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
                        scheduler_context: &self.scheduler_context,
                        llama_context: &self.llama_context,
                    }
                    .run(slice::from_mut(active_request));
                }
                Err(ingestion_error) => {
                    active_request.complete_with_outcome(
                        GenerationRequestRejection::from(ingestion_error)
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

            if active_request.is_stop_requested() {
                let summary = GenerationSummary {
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
            if request
                .generated_embedding_tx
                .send(EmbeddingResult::EmbeddingRejectedDueToActiveTokenGeneration)
                .is_err()
            {
                warn!(
                    "{:?}: failed to send result to client (receiver dropped)",
                    self.scheduler_context.agent_name
                );
            }

            return;
        }

        let mut processor = ContinuousBatchEmbeddingProcessor::new(
            &mut self.batch,
            &mut self.llama_context,
            &self.scheduler_context,
        );

        if let Err(err) = processor.process_embedding_batch(request) {
            error!(
                "{:?}: failed to process embedding batch: {err:#}",
                self.scheduler_context.agent_name
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

    fn execute_one_iteration(&mut self) -> Result<()> {
        self.advance_generating_requests();
        self.ingest_next_multimodal_chunks();

        let n_batch = self.scheduler_context.inference_parameters.n_batch;
        let assemble_phase = AssembleBatchPhase { n_batch };

        loop {
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

            match decode_batch_phase::run(&mut pass, &mut self.llama_context) {
                DecodeOutcome::Decoded => {
                    commit_phase::run(pass, &mut self.active_requests)?;

                    return Ok(());
                }
                DecodeOutcome::NeedsEviction => {
                    self.evict_largest_sequence();

                    if self.active_requests.is_empty() {
                        return Ok(());
                    }
                }
                DecodeOutcome::Aborted => {
                    return Ok(());
                }
                DecodeOutcome::Errored(decode_error) => {
                    return Err(anyhow::Error::new(decode_error).context("decode failed"));
                }
            }
        }
    }

    fn advance_generating_requests(&mut self) {
        AdvanceGeneratingPhase {
            scheduler_context: &self.scheduler_context,
            llama_context: &self.llama_context,
        }
        .run(&mut self.active_requests);
    }

    fn evict_largest_sequence(&mut self) {
        let mut largest_seq_index: Option<usize> = None;
        let mut largest_position: i32 = -1;

        for (index, active_request) in self.active_requests.iter().enumerate() {
            if matches!(
                active_request.state.phase,
                ContinuousBatchRequestPhase::Completed(_)
            ) {
                continue;
            }

            if active_request.state.current_token_position > largest_position {
                largest_position = active_request.state.current_token_position;
                largest_seq_index = Some(index);
            }
        }

        if let Some(eviction_index) = largest_seq_index {
            let evicted_request = &mut self.active_requests[eviction_index];

            warn!(
                "{:?}: evicting sequence {} (position {}) due to KV cache pressure",
                self.scheduler_context.agent_name,
                evicted_request.sequence_id_guard.sequence_id(),
                evicted_request.state.current_token_position
            );

            evicted_request.complete_with_outcome(GeneratedTokenResult::SamplerError(
                "Request evicted due to KV cache pressure".to_owned(),
            ));

            self.cleanup_completed_request(eviction_index);
        }
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

    fn clear_kv_cache_for_sequence(&mut self, sequence_id: i32) {
        let sequence_id_u32 = match u32::try_from(sequence_id) {
            Ok(sequence_id_u32) => sequence_id_u32,
            Err(err) => {
                error!(
                    "{:?}: sequence id {sequence_id} does not fit in u32: {err}",
                    self.scheduler_context.agent_name
                );

                return;
            }
        };

        if let Err(err) = self
            .llama_context
            .clear_kv_cache_seq(Some(sequence_id_u32), None, None)
        {
            error!(
                "{:?}: failed to clear KV cache for sequence {sequence_id}: {err}",
                self.scheduler_context.agent_name
            );
        }
    }

    fn cleanup_completed_request(&mut self, index: usize) {
        let removed_request = self.active_requests.remove(index);
        let sequence_id = removed_request.sequence_id_guard.sequence_id();
        let usage = *removed_request.token_classifier.usage();
        let terminal_delivery = removed_request.into_terminal_delivery();

        self.clear_kv_cache_for_sequence(sequence_id);

        debug!(
            "{:?}: cleaned up sequence {sequence_id} ({} completion tokens generated)",
            self.scheduler_context.agent_name,
            usage.content_tokens + usage.reasoning_tokens + usage.undeterminable_tokens,
        );

        terminal_delivery.deliver(self.scheduler_context.agent_name.as_deref());
    }
}
