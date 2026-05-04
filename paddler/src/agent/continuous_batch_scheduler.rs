use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::DecodeError;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdInputText;
use llama_cpp_bindings::sampling::LlamaSampler;
use log::debug;
use log::error;
use log::info;
use log::warn;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use rand::Rng as _;
use rand::rngs::ThreadRng;
use tokio::sync::mpsc;

use crate::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_embedding_processor::ContinuousBatchEmbeddingProcessor;
use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::agent::grammar_sampler::GrammarSampler;
use crate::agent::prepare_conversation_history_request::prepare_conversation_history_request;
use crate::agent::prepared_conversation_history_request::PreparedConversationHistoryRequest;
use crate::agent::resolve_grammar::resolve_grammar;
use crate::agent::sample_token_at_batch_index::sample_token_at_batch_index;
use crate::agent::sampling_outcome::SamplingOutcome;
use crate::agent::sequence_id_pool::SequenceIdPool;
use crate::decoded_image::DecodedImage;
use crate::dispenses_slots::DispensesSlots;
use crate::slot_aggregated_status::SlotAggregatedStatus;

struct GeneratingContribution {
    request_index: usize,
    batch_position: i32,
}

struct IngestingContribution {
    request_index: usize,
    chunk_size: usize,
    is_last_chunk: bool,
    last_batch_position: i32,
}

pub struct ContinuousBatchScheduler {
    active_requests: Vec<ContinuousBatchActiveRequest>,
    command_rx: Receiver<ContinuousBatchSchedulerCommand>,
    llama_context: LlamaContext<'static>,
    pending_embedding_requests: VecDeque<GenerateEmbeddingBatchRequest>,
    rng: ThreadRng,
    running: bool,
    scheduler_context: Arc<ContinuousBatchSchedulerContext>,
    sequence_id_pool: SequenceIdPool,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl ContinuousBatchScheduler {
    #[expect(
        unsafe_code,
        reason = "required for FFI lifetime extension with llama.cpp"
    )]
    pub fn new(
        command_rx: Receiver<ContinuousBatchSchedulerCommand>,
        scheduler_context: Arc<ContinuousBatchSchedulerContext>,
        llama_context: LlamaContext,
        max_concurrent_sequences: i32,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Self {
        let llama_context = unsafe {
            std::mem::transmute::<LlamaContext<'_>, LlamaContext<'static>>(llama_context)
        };

        Self {
            active_requests: Vec::new(),
            command_rx,
            llama_context,
            pending_embedding_requests: VecDeque::new(),
            rng: rand::rng(),
            running: true,
            scheduler_context,
            sequence_id_pool: SequenceIdPool::new(max_concurrent_sequences),
            slot_aggregated_status,
        }
    }

    pub fn run(&mut self) {
        info!(
            "{:?}: continuous batch scheduler started",
            self.scheduler_context.agent_name
        );

        while self.running {
            self.remove_completed_requests();
            self.accept_new_commands();
            self.check_stop_signals();
            self.try_process_embedding_request();

            if self.has_active_requests() {
                if let Err(err) = self.execute_one_iteration() {
                    error!(
                        "{:?}: scheduler iteration failed: {err:#}",
                        self.scheduler_context.agent_name
                    );
                }
            } else {
                match self.command_rx.recv_timeout(Duration::from_millis(10)) {
                    Ok(command) => self.process_command(command),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        info!(
                            "{:?}: command channel closed, shutting down scheduler",
                            self.scheduler_context.agent_name
                        );
                        self.running = false;
                    }
                }
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

    fn accept_new_commands(&mut self) {
        loop {
            match self.command_rx.try_recv() {
                Ok(command) => self.process_command(command),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.running = false;

                    break;
                }
            }
        }
    }

    fn process_command(&mut self, command: ContinuousBatchSchedulerCommand) {
        match command {
            ContinuousBatchSchedulerCommand::ContinueFromConversationHistory(request) => {
                self.accept_conversation_history_request(request);
            }
            ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(request) => {
                self.accept_raw_prompt_request(request);
            }
            ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(request) => {
                self.pending_embedding_requests.push_back(request);
            }
            ContinuousBatchSchedulerCommand::Shutdown => {
                self.running = false;
            }
        }
    }

    fn accept_conversation_history_request(
        &mut self,
        request: ContinueFromConversationHistoryRequest,
    ) {
        let generated_tokens_tx = request.generated_tokens_tx;
        let generate_tokens_stop_rx = request.generate_tokens_stop_rx;

        let prepared = match prepare_conversation_history_request(
            request.params,
            &generated_tokens_tx,
            &self.scheduler_context,
        ) {
            Ok(prepared) => prepared,
            Err(err) => {
                error!(
                    "{:?}: failed to prepare conversation history request: {err}",
                    self.scheduler_context.agent_name
                );

                return;
            }
        };

        match prepared {
            PreparedConversationHistoryRequest::TextPrompt {
                raw_prompt,
                max_tokens,
                grammar_sampler,
            } => {
                self.accept_text_prompt(
                    &raw_prompt,
                    max_tokens,
                    grammar_sampler,
                    generated_tokens_tx,
                    generate_tokens_stop_rx,
                );
            }
            PreparedConversationHistoryRequest::MultimodalPrompt {
                raw_prompt,
                images,
                max_tokens,
                grammar_sampler,
            } => {
                let multimodal_context = self.scheduler_context.multimodal_context.clone();

                if let Some(multimodal_context) = multimodal_context.as_ref() {
                    self.accept_multimodal_request(
                        multimodal_context,
                        raw_prompt,
                        &images,
                        max_tokens,
                        grammar_sampler,
                        generated_tokens_tx,
                        generate_tokens_stop_rx,
                    );
                }
            }
        }
    }

    fn accept_raw_prompt_request(
        &mut self,
        ContinueFromRawPromptRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params:
                ContinueFromRawPromptParams {
                    grammar,
                    max_tokens,
                    raw_prompt,
                },
        }: ContinueFromRawPromptRequest,
    ) {
        let grammar_sampler = match resolve_grammar(grammar.as_ref(), false, &generated_tokens_tx) {
            Ok(sampler) => sampler,
            Err(err) => {
                error!(
                    "{:?}: failed to resolve grammar: {err}",
                    self.scheduler_context.agent_name
                );

                return;
            }
        };

        self.accept_text_prompt(
            &raw_prompt,
            max_tokens,
            grammar_sampler,
            generated_tokens_tx,
            generate_tokens_stop_rx,
        );
    }

    fn create_sampler_chain(&mut self) -> LlamaSampler {
        LlamaSampler::chain_simple([
            LlamaSampler::penalties(
                self.scheduler_context.inference_parameters.penalty_last_n,
                self.scheduler_context.inference_parameters.penalty_repeat,
                self.scheduler_context
                    .inference_parameters
                    .penalty_frequency,
                self.scheduler_context.inference_parameters.penalty_presence,
            ),
            LlamaSampler::top_k(self.scheduler_context.inference_parameters.top_k),
            LlamaSampler::top_p(self.scheduler_context.inference_parameters.top_p, 0),
            LlamaSampler::min_p(self.scheduler_context.inference_parameters.min_p, 0),
            LlamaSampler::temp(self.scheduler_context.inference_parameters.temperature),
            LlamaSampler::dist(self.rng.random::<u32>()),
        ])
    }

    fn create_grammar_llama_sampler(
        &self,
        grammar_sampler: Option<GrammarSampler>,
        generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    ) -> Result<Option<LlamaSampler>> {
        grammar_sampler.map_or_else(
            || Ok(None),
            |grammar_sampler| match grammar_sampler
                .into_llama_sampler(&self.scheduler_context.model)
            {
                Ok(sampler) => Ok(Some(sampler)),
                Err(err) => {
                    let message = format!(
                        "{:?}: failed to initialize grammar sampler: {err}",
                        self.scheduler_context.agent_name
                    );

                    error!("{message}");

                    if generated_tokens_tx
                        .send(GeneratedTokenResult::GrammarInitializationFailed(
                            message.clone(),
                        ))
                        .is_err()
                    {
                        warn!(
                            "{:?}: failed to send result to client (receiver dropped)",
                            self.scheduler_context.agent_name
                        );
                    }

                    Err(anyhow!(message))
                }
            },
        )
    }

    fn accept_text_prompt(
        &mut self,
        prompt: &str,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
        generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
        generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    ) {
        let mut sequence_id_option = self.sequence_id_pool.acquire();

        if sequence_id_option.is_none() {
            self.remove_completed_requests();
            sequence_id_option = self.sequence_id_pool.acquire();
        }

        let Some(sequence_id) = sequence_id_option else {
            let message = format!(
                "{:?}: no available sequence slots, all slots are busy",
                self.scheduler_context.agent_name
            );

            error!("{message}");

            if generated_tokens_tx
                .send(GeneratedTokenResult::SamplerError(message))
                .is_err()
            {
                warn!(
                    "{:?}: failed to send result to client (receiver dropped)",
                    self.scheduler_context.agent_name
                );
            }

            return;
        };

        let prompt_tokens = match self
            .scheduler_context
            .model
            .str_to_token(prompt, AddBos::Always)
        {
            Ok(tokens) => tokens,
            Err(err) => {
                let message = format!(
                    "{:?}: failed to tokenize prompt: {err}",
                    self.scheduler_context.agent_name
                );

                error!("{message}");
                self.sequence_id_pool.release(sequence_id);

                if generated_tokens_tx
                    .send(GeneratedTokenResult::SamplerError(message))
                    .is_err()
                {
                    warn!(
                        "{:?}: failed to send result to client (receiver dropped)",
                        self.scheduler_context.agent_name
                    );
                }

                return;
            }
        };

        let Ok(llama_grammar_sampler) =
            self.create_grammar_llama_sampler(grammar_sampler, &generated_tokens_tx)
        else {
            self.sequence_id_pool.release(sequence_id);

            return;
        };

        let chain = self.create_sampler_chain();

        #[expect(
            clippy::cast_sign_loss,
            reason = "sequence IDs are always non-negative"
        )]
        if let Err(err) =
            self.llama_context
                .clear_kv_cache_seq(Some(sequence_id as u32), None, None)
        {
            error!(
                "{:?}: failed to clear KV cache for sequence {sequence_id}: {err}",
                self.scheduler_context.agent_name
            );
        }

        self.slot_aggregated_status.take_slot();

        debug!(
            "{:?}: accepted text prompt request on sequence {sequence_id} ({} tokens)",
            self.scheduler_context.agent_name,
            prompt_tokens.len()
        );

        self.active_requests.push(ContinuousBatchActiveRequest {
            chain,
            current_token_position: 0,
            grammar_sampler: llama_grammar_sampler,
            generated_tokens_count: 0,
            generated_tokens_tx,
            generate_tokens_stop_rx,
            i_batch: None,
            max_tokens,
            pending_sampled_token: None,
            phase: ContinuousBatchRequestPhase::Ingesting,
            prompt_tokens,
            prompt_tokens_ingested: 0,
            sequence_id,
            utf8_decoder: encoding_rs::UTF_8.new_decoder(),
        });
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "multimodal request handling genuinely requires all these parameters from the caller"
    )]
    fn accept_multimodal_request(
        &mut self,
        multimodal_context: &MtmdContext,
        prompt: String,
        images: &[DecodedImage],
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
        generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
        generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    ) {
        let Some(sequence_id) = self.sequence_id_pool.acquire() else {
            let message = format!(
                "{:?}: no available sequence slots for multimodal request",
                self.scheduler_context.agent_name
            );

            error!("{message}");

            if generated_tokens_tx
                .send(GeneratedTokenResult::SamplerError(message))
                .is_err()
            {
                warn!(
                    "{:?}: failed to send result to client (receiver dropped)",
                    self.scheduler_context.agent_name
                );
            }

            return;
        };

        let bitmaps: Vec<MtmdBitmap> = match images
            .iter()
            .map(|image| {
                MtmdBitmap::from_buffer(multimodal_context, &image.data)
                    .map_err(|err| anyhow!("Failed to create bitmap: {err}"))
            })
            .collect::<Result<Vec<_>>>()
        {
            Ok(bitmaps) => bitmaps,
            Err(err) => {
                let message = format!(
                    "{:?}: failed to create bitmaps: {err}",
                    self.scheduler_context.agent_name
                );

                error!("{message}");
                self.sequence_id_pool.release(sequence_id);

                if generated_tokens_tx
                    .send(GeneratedTokenResult::ImageDecodingFailed(message))
                    .is_err()
                {
                    warn!(
                        "{:?}: failed to send result to client (receiver dropped)",
                        self.scheduler_context.agent_name
                    );
                }

                return;
            }
        };

        let bitmap_refs: Vec<&MtmdBitmap> = bitmaps.iter().collect();

        let input_text = MtmdInputText {
            text: prompt,
            add_special: true,
            parse_special: true,
        };

        let input_chunks = match multimodal_context
            .tokenize(input_text, &bitmap_refs)
            .map_err(|err| anyhow!("Failed to tokenize multimodal input: {err}"))
        {
            Ok(chunks) => chunks,
            Err(err) => {
                let message = format!(
                    "{:?}: failed to tokenize multimodal input: {err}",
                    self.scheduler_context.agent_name
                );

                error!("{message}");
                self.sequence_id_pool.release(sequence_id);

                if generated_tokens_tx
                    .send(GeneratedTokenResult::SamplerError(message))
                    .is_err()
                {
                    warn!(
                        "{:?}: failed to send result to client (receiver dropped)",
                        self.scheduler_context.agent_name
                    );
                }

                return;
            }
        };

        let batch_size = self.scheduler_context.inference_parameters.batch_n_tokens;

        #[expect(
            clippy::cast_sign_loss,
            reason = "sequence IDs are always non-negative"
        )]
        if let Err(err) =
            self.llama_context
                .clear_kv_cache_seq(Some(sequence_id as u32), None, None)
        {
            error!(
                "{:?}: failed to clear KV cache for sequence {sequence_id}: {err}",
                self.scheduler_context.agent_name
            );
        }

        self.harvest_pending_samples_before_external_decode();

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "batch_size fits in i32 for llama.cpp FFI"
        )]
        let tokens_ingested = match input_chunks
            .eval_chunks(
                multimodal_context,
                &self.llama_context,
                0,
                sequence_id,
                batch_size as i32,
                true,
            )
            .map_err(|err| anyhow!("Failed to evaluate multimodal chunks: {err}"))
        {
            Ok(tokens_ingested) => tokens_ingested,
            Err(err) => {
                let message = format!(
                    "{:?}: failed to ingest multimodal prompt: {err}",
                    self.scheduler_context.agent_name
                );

                error!("{message}");
                self.sequence_id_pool.release(sequence_id);

                if generated_tokens_tx
                    .send(GeneratedTokenResult::SamplerError(message))
                    .is_err()
                {
                    warn!(
                        "{:?}: failed to send result to client (receiver dropped)",
                        self.scheduler_context.agent_name
                    );
                }

                return;
            }
        };

        self.llama_context.mark_logits_initialized(-1);

        let Ok(llama_grammar_sampler) =
            self.create_grammar_llama_sampler(grammar_sampler, &generated_tokens_tx)
        else {
            self.sequence_id_pool.release(sequence_id);

            return;
        };

        let chain = self.create_sampler_chain();

        self.slot_aggregated_status.take_slot();

        debug!(
            "{:?}: accepted multimodal request on sequence {sequence_id} ({tokens_ingested} tokens ingested)",
            self.scheduler_context.agent_name
        );

        self.active_requests.push(ContinuousBatchActiveRequest {
            chain,
            current_token_position: tokens_ingested,
            grammar_sampler: llama_grammar_sampler,
            generated_tokens_count: 0,
            generated_tokens_tx,
            generate_tokens_stop_rx,
            i_batch: Some(-1),
            max_tokens,
            pending_sampled_token: None,
            phase: ContinuousBatchRequestPhase::Generating,
            prompt_tokens: Vec::new(),
            prompt_tokens_ingested: 0,
            sequence_id,
            utf8_decoder: encoding_rs::UTF_8.new_decoder(),
        });
    }

    fn harvest_pending_samples_before_external_decode(&mut self) {
        for active_request in &mut self.active_requests {
            if !matches!(
                active_request.phase,
                ContinuousBatchRequestPhase::Generating
            ) {
                continue;
            }

            if active_request.pending_sampled_token.is_some() {
                continue;
            }

            let Some(batch_index) = active_request.i_batch else {
                continue;
            };

            match sample_token_at_batch_index(
                &self.llama_context,
                batch_index,
                &mut active_request.chain,
                &mut active_request.grammar_sampler,
            ) {
                Ok(SamplingOutcome::Token(sampled_token)) => {
                    active_request.pending_sampled_token = Some(sampled_token);
                    active_request.i_batch = None;
                }
                Ok(SamplingOutcome::AllCandidatesEliminated) => {
                    error!(
                        "{:?}: sequence {} pre-eval harvest exhausted candidates",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );
                    active_request.complete_with_outcome(
                        &self.scheduler_context.agent_name,
                        GeneratedTokenResult::SamplerError(
                            "all token candidates were eliminated during sampling".to_owned(),
                        ),
                    );
                }
                Ok(SamplingOutcome::GrammarRejectedModelOutput(message)) => {
                    error!(
                        "{:?}: sequence {} pre-eval harvest grammar rejected: {message}",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );
                    active_request.complete_with_outcome(
                        &self.scheduler_context.agent_name,
                        GeneratedTokenResult::GrammarRejectedModelOutput(message),
                    );
                }
                Err(err) => {
                    error!(
                        "{:?}: sequence {} pre-eval harvest sampling error: {err:#}",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );
                    active_request.complete_with_outcome(
                        &self.scheduler_context.agent_name,
                        GeneratedTokenResult::SamplerError(err.to_string()),
                    );
                }
            }
        }
    }

    fn check_stop_signals(&mut self) {
        for active_request in &mut self.active_requests {
            if active_request.is_stop_requested() {
                active_request.complete_with_outcome(
                    &self.scheduler_context.agent_name,
                    GeneratedTokenResult::Done,
                );
            }
        }
    }

    fn try_process_embedding_request(&mut self) {
        let Some(request) = self.pending_embedding_requests.pop_front() else {
            return;
        };

        if self.has_active_requests() {
            if request
                .generated_embedding_tx
                .send(EmbeddingResult::Error(
                    "Embedding requests cannot be processed while generation requests are active"
                        .to_owned(),
                ))
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
        self.active_requests
            .iter()
            .any(|request| !matches!(request.phase, ContinuousBatchRequestPhase::Completed))
    }

    fn execute_one_iteration(&mut self) -> Result<()> {
        self.advance_generating_requests();

        let batch_n_tokens = self.scheduler_context.inference_parameters.batch_n_tokens;

        loop {
            let max_sequences = self.active_requests.len();

            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                reason = "token counts and positions fit in i32 for llama.cpp FFI"
            )]
            let mut batch = LlamaBatch::new(batch_n_tokens, max_sequences.max(1) as i32)?;

            let mut generating_contributions: Vec<GeneratingContribution> = Vec::new();
            let mut ingesting_contributions: Vec<IngestingContribution> = Vec::new();

            let mut current_batch_token_count: usize = 0;

            current_batch_token_count += self.add_generating_pending_tokens_to_batch(
                &mut batch,
                batch_n_tokens,
                &mut generating_contributions,
            )?;

            self.add_ingesting_prompt_chunks_to_batch(
                &mut batch,
                batch_n_tokens,
                current_batch_token_count,
                &mut ingesting_contributions,
            )?;

            if batch.n_tokens() == 0 {
                return Ok(());
            }

            debug!(
                "{:?}: decoding batch with {} tokens for {} active requests",
                self.scheduler_context.agent_name,
                batch.n_tokens(),
                self.active_requests.len()
            );

            match self.llama_context.decode(&mut batch) {
                Ok(()) => {
                    self.commit_contributions(&generating_contributions, &ingesting_contributions);

                    return Ok(());
                }
                Err(DecodeError::NoKvCacheSlot) => {
                    self.evict_largest_sequence();

                    if self.active_requests.is_empty() {
                        return Ok(());
                    }
                }
                Err(DecodeError::Aborted | DecodeError::NTokensZero) => {
                    return Ok(());
                }
                Err(DecodeError::Unknown(error_code)) => {
                    return Err(anyhow!(
                        "Decode failed with unknown error code: {error_code}"
                    ));
                }
            }
        }
    }

    fn advance_generating_requests(&mut self) {
        for active_request in &mut self.active_requests {
            if !matches!(
                active_request.phase,
                ContinuousBatchRequestPhase::Generating
            ) {
                continue;
            }

            if active_request.pending_sampled_token.is_some() {
                continue;
            }

            let Some(batch_index) = active_request.i_batch else {
                continue;
            };

            let sampled_token = match sample_token_at_batch_index(
                &self.llama_context,
                batch_index,
                &mut active_request.chain,
                &mut active_request.grammar_sampler,
            ) {
                Ok(SamplingOutcome::Token(sampled_token)) => sampled_token,
                Ok(SamplingOutcome::AllCandidatesEliminated) => {
                    error!(
                        "{:?}: sequence {} sampling exhausted candidates",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );
                    active_request.complete_with_outcome(
                        &self.scheduler_context.agent_name,
                        GeneratedTokenResult::SamplerError(
                            "all token candidates were eliminated during sampling".to_owned(),
                        ),
                    );
                    continue;
                }
                Ok(SamplingOutcome::GrammarRejectedModelOutput(message)) => {
                    error!(
                        "{:?}: sequence {} grammar rejected sampled token: {message}",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );
                    active_request.complete_with_outcome(
                        &self.scheduler_context.agent_name,
                        GeneratedTokenResult::GrammarRejectedModelOutput(message),
                    );
                    continue;
                }
                Err(err) => {
                    error!(
                        "{:?}: sequence {} sampling error: {err:#}",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );
                    active_request.complete_with_outcome(
                        &self.scheduler_context.agent_name,
                        GeneratedTokenResult::SamplerError(err.to_string()),
                    );
                    continue;
                }
            };

            if self.scheduler_context.model.is_eog_token(sampled_token) {
                active_request.complete_with_outcome(
                    &self.scheduler_context.agent_name,
                    GeneratedTokenResult::Done,
                );
                continue;
            }

            let output_string = match self.scheduler_context.model.token_to_piece(
                sampled_token,
                &mut active_request.utf8_decoder,
                true,
                None,
            ) {
                Ok(output_string) => output_string,
                Err(err) => {
                    error!(
                        "{:?}: sequence {} token_to_piece failed: {err}",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );
                    active_request.complete_with_outcome(
                        &self.scheduler_context.agent_name,
                        GeneratedTokenResult::SamplerError(format!(
                            "Failed to convert token to string: {err}"
                        )),
                    );
                    continue;
                }
            };

            if active_request
                .generated_tokens_tx
                .send(GeneratedTokenResult::Token(output_string))
                .is_err()
            {
                warn!(
                    "{:?}: sequence {} client disconnected (receiver dropped)",
                    self.scheduler_context.agent_name, active_request.sequence_id
                );

                active_request.i_batch = None;
                active_request.phase = ContinuousBatchRequestPhase::Completed;

                continue;
            }

            active_request.generated_tokens_count += 1;

            if active_request.generated_tokens_count >= active_request.max_tokens {
                active_request.complete_with_outcome(
                    &self.scheduler_context.agent_name,
                    GeneratedTokenResult::Done,
                );
                continue;
            }

            active_request.pending_sampled_token = Some(sampled_token);
        }
    }

    fn add_generating_pending_tokens_to_batch(
        &self,
        batch: &mut LlamaBatch,
        batch_n_tokens: usize,
        contributions: &mut Vec<GeneratingContribution>,
    ) -> Result<usize> {
        let mut tokens_added: usize = 0;

        for (request_index, active_request) in self.active_requests.iter().enumerate() {
            if !matches!(
                active_request.phase,
                ContinuousBatchRequestPhase::Generating
            ) {
                continue;
            }

            let Some(pending_token) = active_request.pending_sampled_token else {
                continue;
            };

            if tokens_added >= batch_n_tokens {
                break;
            }

            let batch_position = batch.n_tokens();

            batch.add(
                pending_token,
                active_request.current_token_position,
                &[active_request.sequence_id],
                true,
            )?;

            contributions.push(GeneratingContribution {
                request_index,
                batch_position,
            });

            tokens_added += 1;
        }

        Ok(tokens_added)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "token counts and positions fit in i32 for llama.cpp FFI"
    )]
    fn add_ingesting_prompt_chunks_to_batch(
        &self,
        batch: &mut LlamaBatch,
        batch_n_tokens: usize,
        mut current_batch_token_count: usize,
        contributions: &mut Vec<IngestingContribution>,
    ) -> Result<()> {
        for (request_index, active_request) in self.active_requests.iter().enumerate() {
            if !matches!(active_request.phase, ContinuousBatchRequestPhase::Ingesting) {
                continue;
            }

            let remaining = active_request.remaining_prompt_tokens();
            let available_space = batch_n_tokens.saturating_sub(current_batch_token_count);
            let chunk_size = remaining.len().min(available_space);

            if chunk_size == 0 {
                continue;
            }

            let chunk = &active_request.prompt_tokens[active_request.prompt_tokens_ingested
                ..active_request.prompt_tokens_ingested + chunk_size];
            let is_last_chunk = active_request.prompt_tokens_ingested + chunk_size
                >= active_request.prompt_tokens.len();

            for (offset, token) in chunk.iter().enumerate() {
                let position = active_request.current_token_position + offset as i32;
                let is_last_token_of_prompt = is_last_chunk && offset == chunk_size - 1;

                batch.add(
                    *token,
                    position,
                    &[active_request.sequence_id],
                    is_last_token_of_prompt,
                )?;
            }

            contributions.push(IngestingContribution {
                request_index,
                chunk_size,
                is_last_chunk,
                last_batch_position: batch.n_tokens() - 1,
            });

            current_batch_token_count += chunk_size;
        }

        Ok(())
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "chunk sizes fit in i32 for llama.cpp position arithmetic"
    )]
    fn commit_contributions(
        &mut self,
        generating_contributions: &[GeneratingContribution],
        ingesting_contributions: &[IngestingContribution],
    ) {
        for contribution in generating_contributions {
            let request = &mut self.active_requests[contribution.request_index];

            request.pending_sampled_token = None;
            request.i_batch = Some(contribution.batch_position);
            request.current_token_position += 1;
        }

        for contribution in ingesting_contributions {
            let request = &mut self.active_requests[contribution.request_index];

            request.prompt_tokens_ingested += contribution.chunk_size;
            request.current_token_position += contribution.chunk_size as i32;

            if contribution.is_last_chunk {
                request.i_batch = Some(contribution.last_batch_position);
                request.phase = ContinuousBatchRequestPhase::Generating;
            }
        }
    }

    fn evict_largest_sequence(&mut self) {
        let mut largest_seq_index: Option<usize> = None;
        let mut largest_position: i32 = -1;

        for (index, active_request) in self.active_requests.iter().enumerate() {
            if matches!(active_request.phase, ContinuousBatchRequestPhase::Completed) {
                continue;
            }

            if active_request.current_token_position > largest_position {
                largest_position = active_request.current_token_position;
                largest_seq_index = Some(index);
            }
        }

        if let Some(eviction_index) = largest_seq_index {
            let evicted_request = &mut self.active_requests[eviction_index];

            warn!(
                "{:?}: evicting sequence {} (position {}) due to KV cache pressure",
                self.scheduler_context.agent_name,
                evicted_request.sequence_id,
                evicted_request.current_token_position
            );

            if evicted_request
                .generated_tokens_tx
                .send(GeneratedTokenResult::SamplerError(
                    "Request evicted due to KV cache pressure".to_owned(),
                ))
                .is_err()
            {
                warn!(
                    "{:?}: failed to send result to client (receiver dropped)",
                    self.scheduler_context.agent_name
                );
            }

            evicted_request.phase = ContinuousBatchRequestPhase::Completed;

            self.cleanup_completed_request(eviction_index);
        }
    }

    fn remove_completed_requests(&mut self) {
        let mut removal_index = 0;

        while removal_index < self.active_requests.len() {
            if matches!(
                self.active_requests[removal_index].phase,
                ContinuousBatchRequestPhase::Completed
            ) {
                self.cleanup_completed_request(removal_index);
            } else {
                removal_index += 1;
            }
        }
    }

    fn cleanup_completed_request(&mut self, index: usize) {
        let removed_request = self.active_requests.swap_remove(index);

        #[expect(
            clippy::cast_sign_loss,
            reason = "sequence IDs are always non-negative"
        )]
        if let Err(err) = self.llama_context.clear_kv_cache_seq(
            Some(removed_request.sequence_id as u32),
            None,
            None,
        ) {
            error!(
                "{:?}: failed to clear KV cache for sequence {}: {err}",
                self.scheduler_context.agent_name, removed_request.sequence_id
            );
        }

        self.sequence_id_pool.release(removed_request.sequence_id);
        self.slot_aggregated_status.release_slot();

        debug!(
            "{:?}: cleaned up sequence {} ({} tokens generated)",
            self.scheduler_context.agent_name,
            removed_request.sequence_id,
            removed_request.generated_tokens_count,
        );
    }
}
