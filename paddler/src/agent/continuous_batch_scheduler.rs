use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::DecodeError;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdInputText;
use llama_cpp_bindings::mtmd::mtmd_default_marker;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::LlamaToken;
use log::debug;
use log::error;
use log::info;
use log::warn;
use minijinja::context;
use paddler_types::embedding::Embedding;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::grammar_constraint::GrammarConstraint;
use paddler_types::media_marker::MediaMarker;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use rand::Rng as _;
use rand::rngs::ThreadRng;
use tokio::sync::mpsc;

use crate::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::agent::grammar_sampler::GrammarSampler;
use crate::agent::sequence_id_pool::SequenceIdPool;
use crate::decoded_image::DecodedImage;
use crate::decoded_image_error::DecodedImageError;
use crate::dispenses_slots::DispensesSlots;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::slot_aggregated_status::SlotAggregatedStatus;

fn sample_token_at_batch_index(
    llama_context: &LlamaContext,
    batch_index: i32,
    chain: &mut LlamaSampler,
    grammar_sampler: &mut Option<LlamaSampler>,
) -> Result<LlamaToken, GeneratedTokenResult> {
    let mut token_data_array = llama_context
        .token_data_array_ith(batch_index)
        .map_err(|err| GeneratedTokenResult::SamplerError(err.to_string()))?;

    if let Some(grammar) = grammar_sampler.as_ref() {
        token_data_array.apply_sampler(grammar);
    }

    token_data_array.apply_sampler(chain);

    let token = token_data_array.selected_token().ok_or_else(|| {
        GeneratedTokenResult::SamplerError(
            "all token candidates were eliminated during sampling".to_owned(),
        )
    })?;

    chain
        .accept(token)
        .map_err(|err| GeneratedTokenResult::SamplerError(err.to_string()))?;

    if let Some(grammar) = grammar_sampler.as_mut() {
        grammar
            .accept(token)
            .map_err(|err| GeneratedTokenResult::GrammarRejectedModelOutput(err.to_string()))?;
    }

    Ok(token)
}

fn resolve_grammar(
    grammar: Option<&GrammarConstraint>,
    enable_thinking: bool,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
) -> Result<Option<GrammarSampler>> {
    let Some(grammar_constraint) = grammar else {
        return Ok(None);
    };

    if enable_thinking {
        let message = "Grammar constraints are incompatible with thinking mode".to_owned();

        generated_tokens_tx
            .send(GeneratedTokenResult::GrammarIncompatibleWithThinking(
                message.clone(),
            ))
            .map_err(|err| anyhow!("Failed to send grammar incompatibility error: {err}"))?;

        return Err(anyhow!(message));
    }

    match GrammarSampler::new(grammar_constraint) {
        Ok(sampler) => Ok(Some(sampler)),
        Err(err) => {
            let message = format!("Failed to create grammar sampler: {err}");

            generated_tokens_tx
                .send(GeneratedTokenResult::GrammarSyntaxError(message.clone()))
                .map_err(|send_err| anyhow!("Failed to send grammar syntax error: {send_err}"))?;

            Err(anyhow!(message))
        }
    }
}

pub struct ContinuousBatchScheduler {
    active_requests: Vec<ContinuousBatchActiveRequest>,
    command_rx: Receiver<ContinuousBatchSchedulerCommand>,
    llama_context: LlamaContext<'static>,
    pending_embedding_requests: Vec<GenerateEmbeddingBatchRequest>,
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
            pending_embedding_requests: Vec::new(),
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
                match self.command_rx.recv_timeout(Duration::from_millis(100)) {
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
                self.pending_embedding_requests.push(request);
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

        let ContinueFromConversationHistoryParams {
            add_generation_prompt,
            enable_thinking,
            grammar,
            conversation_history,
            max_tokens,
            tools,
        } = request.params;

        let grammar_sampler =
            match resolve_grammar(grammar.as_ref(), enable_thinking, &generated_tokens_tx) {
                Ok(sampler) => sampler,
                Err(err) => {
                    error!(
                        "{:?}: failed to resolve grammar: {err}",
                        self.scheduler_context.agent_name
                    );

                    return;
                }
            };

        let image_resize_to_fit = self
            .scheduler_context
            .inference_parameters
            .image_resize_to_fit;

        let images = match conversation_history
            .extract_image_urls()
            .iter()
            .map(|image_url| {
                DecodedImage::from_data_uri(image_url)
                    .and_then(|image| image.converted_to_png_if_necessary(image_resize_to_fit))
                    .and_then(|image| image.resized_to_fit(image_resize_to_fit))
            })
            .collect::<Result<Vec<DecodedImage>, DecodedImageError>>()
        {
            Ok(images) => images,
            Err(err) => {
                let message = format!(
                    "{:?}: failed to decode images: {err}",
                    self.scheduler_context.agent_name
                );

                error!("{message}");

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

        let media_marker = MediaMarker::new(mtmd_default_marker().to_owned());
        let chat_template_messages = conversation_history.replace_images_with_marker(&media_marker);

        let raw_prompt = match self
            .scheduler_context
            .chat_template_renderer
            .render(context! {
                add_generation_prompt,
                bos_token => self.scheduler_context.token_bos_str,
                enable_thinking,
                eos_token => self.scheduler_context.token_eos_str,
                messages => chat_template_messages.messages,
                nl_token => self.scheduler_context.token_nl_str,
                tools => tools,
            }) {
            Ok(raw_prompt) => raw_prompt,
            Err(err) => {
                let message = format!(
                    "{:?}: failed to render chat template: {err:?}",
                    self.scheduler_context.agent_name
                );

                error!("{message}");

                if generated_tokens_tx
                    .send(GeneratedTokenResult::ChatTemplateError(message))
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

        let has_images = !images.is_empty();
        let multimodal_context = self.scheduler_context.multimodal_context.clone();

        match multimodal_context.as_ref() {
            Some(multimodal_context) if has_images => {
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
            None if has_images => {
                let message = format!(
                    "{:?}: received images but model does not support multimodal input",
                    self.scheduler_context.agent_name
                );

                error!("{message}");

                if generated_tokens_tx
                    .send(GeneratedTokenResult::MultimodalNotSupported(message))
                    .is_err()
                {
                    warn!(
                        "{:?}: failed to send result to client (receiver dropped)",
                        self.scheduler_context.agent_name
                    );
                }
            }
            _ => {
                self.accept_text_prompt(
                    &raw_prompt,
                    max_tokens,
                    grammar_sampler,
                    generated_tokens_tx,
                    generate_tokens_stop_rx,
                );
            }
        }
    }

    fn accept_raw_prompt_request(&mut self, request: ContinueFromRawPromptRequest) {
        let ContinueFromRawPromptParams {
            grammar,
            max_tokens,
            raw_prompt,
        } = request.params;

        let grammar_sampler =
            match resolve_grammar(grammar.as_ref(), false, &request.generated_tokens_tx) {
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
            request.generated_tokens_tx,
            request.generate_tokens_stop_rx,
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
            phase: ContinuousBatchRequestPhase::Generating,
            prompt_tokens: Vec::new(),
            prompt_tokens_ingested: 0,
            sequence_id,
            utf8_decoder: encoding_rs::UTF_8.new_decoder(),
        });
    }

    fn check_stop_signals(&mut self) {
        for active_request in &mut self.active_requests {
            if active_request.is_stop_requested() {
                if active_request
                    .generated_tokens_tx
                    .send(GeneratedTokenResult::Done)
                    .is_err()
                {
                    warn!(
                        "{:?}: failed to send stop Done to client (receiver dropped)",
                        self.scheduler_context.agent_name
                    );
                }

                active_request.i_batch = None;
                active_request.phase = ContinuousBatchRequestPhase::Completed;
            }
        }
    }

    fn try_process_embedding_request(&mut self) {
        if self.pending_embedding_requests.is_empty() {
            return;
        }

        if self.has_active_requests() {
            let request = self.pending_embedding_requests.remove(0);

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

        let request = self.pending_embedding_requests.remove(0);

        if let Err(err) = self.process_embedding_batch(request) {
            error!(
                "{:?}: failed to process embedding batch: {err:#}",
                self.scheduler_context.agent_name
            );
        }
    }

    fn process_embedding_batch(
        &mut self,
        GenerateEmbeddingBatchRequest {
            mut generate_embedding_stop_rx,
            generated_embedding_tx,
            params:
                GenerateEmbeddingBatchParams {
                    input_batch,
                    normalization_method,
                },
        }: GenerateEmbeddingBatchRequest,
    ) -> Result<()> {
        if !self
            .scheduler_context
            .inference_parameters
            .enable_embeddings
        {
            generated_embedding_tx.send(EmbeddingResult::Error(
                "Embeddings are not enabled for this agent".to_owned(),
            ))?;

            return Err(anyhow!("Embeddings are not enabled"));
        }

        self.llama_context.clear_kv_cache();

        let tokens_lines_list = input_batch
            .into_iter()
            .map(|input| {
                match self
                    .scheduler_context
                    .model
                    .str_to_token(&input.content, AddBos::Always)
                {
                    Ok(llama_tokens) => Ok(EmbeddingInputTokenized {
                        id: input.id,
                        llama_tokens,
                    }),
                    Err(err) => Err(anyhow!("Failed to tokenize input: {err:?}")),
                }
            })
            .collect::<Result<Vec<EmbeddingInputTokenized>, _>>()
            .context("failed to tokenize embedding input batch")?;

        let batch_n_tokens = self.scheduler_context.inference_parameters.batch_n_tokens;

        #[expect(
            clippy::cast_possible_wrap,
            reason = "embedding_n_seq_max fits in i32 for llama.cpp FFI"
        )]
        let embedding_n_seq_max = self
            .scheduler_context
            .inference_parameters
            .embedding_n_seq_max as i32;
        let mut batch = LlamaBatch::new(batch_n_tokens, embedding_n_seq_max)?;
        let mut current_batch_inputs: Vec<&EmbeddingInputTokenized> = Vec::new();
        let mut current_batch_token_count: usize = 0;
        let mut next_seq_id: i32 = 0;

        for embedding_input_tokenized in &tokens_lines_list {
            if generate_embedding_stop_rx.try_recv().is_ok() {
                break;
            }

            let input_token_count = embedding_input_tokenized.llama_tokens.len();

            if (current_batch_token_count + input_token_count > batch_n_tokens
                || next_seq_id >= embedding_n_seq_max)
                && !current_batch_inputs.is_empty()
            {
                self.embedding_batch_decode(
                    &mut batch,
                    &current_batch_inputs,
                    &generated_embedding_tx,
                    &normalization_method,
                )?;

                current_batch_inputs.clear();
                current_batch_token_count = 0;
                next_seq_id = 0;
            }

            batch.add_sequence(&embedding_input_tokenized.llama_tokens, next_seq_id, true)?;

            current_batch_inputs.push(embedding_input_tokenized);
            current_batch_token_count += input_token_count;
            next_seq_id += 1;
        }

        if !current_batch_inputs.is_empty() {
            self.embedding_batch_decode(
                &mut batch,
                &current_batch_inputs,
                &generated_embedding_tx,
                &normalization_method,
            )?;
        }

        generated_embedding_tx.send(EmbeddingResult::Done)?;

        Ok(())
    }

    fn embedding_batch_decode(
        &mut self,
        batch: &mut LlamaBatch,
        current_batch_embeddings: &[&EmbeddingInputTokenized],
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<()> {
        self.llama_context.clear_kv_cache();
        self.llama_context.decode(batch)?;

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "embedding sequence index fits in i32 for llama.cpp FFI"
        )]
        for (index, embedding_input_tokenized) in current_batch_embeddings.iter().enumerate() {
            let embedding = self
                .llama_context
                .embeddings_seq_ith(index as i32)
                .context("Failed to get embeddings")?;

            generated_embedding_tx.send(EmbeddingResult::Embedding(
                Embedding {
                    embedding: embedding.to_vec(),
                    normalization_method: EmbeddingNormalizationMethod::None,
                    pooling_type: self
                        .scheduler_context
                        .inference_parameters
                        .pooling_type
                        .clone(),
                    source_document_id: embedding_input_tokenized.id.clone(),
                }
                .normalize(normalization_method)?,
            ))?;
        }

        batch.clear();

        Ok(())
    }

    fn has_active_requests(&self) -> bool {
        self.active_requests
            .iter()
            .any(|request| !matches!(request.phase, ContinuousBatchRequestPhase::Completed))
    }

    fn execute_one_iteration(&mut self) -> Result<()> {
        let batch_n_tokens = self.scheduler_context.inference_parameters.batch_n_tokens;
        let max_sequences = self.active_requests.len();

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "token counts and positions fit in i32 for llama.cpp FFI"
        )]
        let mut batch = LlamaBatch::new(batch_n_tokens, max_sequences.max(1) as i32)?;

        let mut current_batch_token_count: usize = 0;

        current_batch_token_count +=
            self.sample_generating_requests_into_batch(&mut batch, batch_n_tokens)?;

        self.ingest_prompt_tokens_into_batch(
            &mut batch,
            batch_n_tokens,
            current_batch_token_count,
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

        if let Err(err) = self.llama_context.decode(&mut batch) {
            match err {
                DecodeError::NoKvCacheSlot => {
                    self.evict_largest_sequence();

                    return self.execute_one_iteration();
                }
                DecodeError::Aborted | DecodeError::NTokensZero => {
                    return Ok(());
                }
                DecodeError::Unknown(error_code) => {
                    return Err(anyhow!(
                        "Decode failed with unknown error code: {error_code}"
                    ));
                }
            }
        }

        Ok(())
    }

    fn sample_generating_requests_into_batch(
        &mut self,
        batch: &mut LlamaBatch,
        batch_n_tokens: usize,
    ) -> Result<usize> {
        let mut tokens_added: usize = 0;

        for active_request in &mut self.active_requests {
            if !matches!(
                active_request.phase,
                ContinuousBatchRequestPhase::Generating
            ) {
                continue;
            }

            let Some(batch_index) = active_request.i_batch else {
                continue;
            };

            if tokens_added >= batch_n_tokens {
                break;
            }

            let sampled_token = match sample_token_at_batch_index(
                &self.llama_context,
                batch_index,
                &mut active_request.chain,
                &mut active_request.grammar_sampler,
            ) {
                Ok(token) => token,
                Err(result) => {
                    error!(
                        "{:?}: sequence {} sampling error: {result:?}",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );

                    if active_request.generated_tokens_tx.send(result).is_err() {
                        warn!(
                            "{:?}: failed to send result to client (receiver dropped)",
                            self.scheduler_context.agent_name
                        );
                    }

                    active_request.i_batch = None;
                    active_request.phase = ContinuousBatchRequestPhase::Completed;

                    continue;
                }
            };

            if self.scheduler_context.model.is_eog_token(sampled_token) {
                if active_request
                    .generated_tokens_tx
                    .send(GeneratedTokenResult::Done)
                    .is_err()
                {
                    warn!(
                        "{:?}: failed to send result to client (receiver dropped)",
                        self.scheduler_context.agent_name
                    );
                }

                active_request.i_batch = None;
                active_request.phase = ContinuousBatchRequestPhase::Completed;

                continue;
            }

            match self.scheduler_context.model.token_to_piece(
                sampled_token,
                &mut active_request.utf8_decoder,
                true,
                None,
            ) {
                Ok(output_string) => {
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
                }
                Err(err) => {
                    error!(
                        "{:?}: sequence {} token_to_piece failed: {err}",
                        self.scheduler_context.agent_name, active_request.sequence_id
                    );

                    if active_request
                        .generated_tokens_tx
                        .send(GeneratedTokenResult::SamplerError(format!(
                            "Failed to convert token to string: {err}"
                        )))
                        .is_err()
                    {
                        warn!(
                            "{:?}: failed to send result to client (receiver dropped)",
                            self.scheduler_context.agent_name
                        );
                    }

                    active_request.i_batch = None;
                    active_request.phase = ContinuousBatchRequestPhase::Completed;

                    continue;
                }
            }

            active_request.generated_tokens_count += 1;

            if active_request.generated_tokens_count >= active_request.max_tokens {
                if active_request
                    .generated_tokens_tx
                    .send(GeneratedTokenResult::Done)
                    .is_err()
                {
                    warn!(
                        "{:?}: failed to send result to client (receiver dropped)",
                        self.scheduler_context.agent_name
                    );
                }

                active_request.i_batch = None;
                active_request.phase = ContinuousBatchRequestPhase::Completed;

                continue;
            }

            active_request.i_batch = Some(batch.n_tokens());

            batch.add(
                sampled_token,
                active_request.current_token_position,
                &[active_request.sequence_id],
                true,
            )?;

            active_request.current_token_position += 1;
            tokens_added += 1;
        }

        Ok(tokens_added)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "token counts and positions fit in i32 for llama.cpp FFI"
    )]
    fn ingest_prompt_tokens_into_batch(
        &mut self,
        batch: &mut LlamaBatch,
        batch_n_tokens: usize,
        mut current_batch_token_count: usize,
    ) -> Result<()> {
        for active_request in &mut self.active_requests {
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

            if is_last_chunk {
                active_request.i_batch = Some(batch.n_tokens() - 1);
                active_request.phase = ContinuousBatchRequestPhase::Generating;
            }

            active_request.prompt_tokens_ingested += chunk_size;
            active_request.current_token_position += chunk_size as i32;
            current_batch_token_count += chunk_size;
        }

        Ok(())
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
