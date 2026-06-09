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
pub mod tool_call_pass;
pub mod tool_call_pipeline_build_outcome;

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::EvalMultimodalChunksError;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdEvalError;
use llama_cpp_bindings::mtmd::MtmdInputText;
use llama_cpp_bindings::sampling::LlamaSampler;
use log::debug;
use log::error;
use log::info;
use log::warn;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::oversized_image_details::OversizedImageDetails;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use rand::Rng as _;
use rand::rngs::ThreadRng;
use tokio::sync::mpsc;

use self::advance_generating_phase::AdvanceGeneratingPhase;
use self::assemble_batch_phase::AssembleBatchPhase;
use self::batch_pass::BatchPass;
use self::decode_outcome::DecodeOutcome;
use self::tool_call_pipeline_build_outcome::ToolCallPipelineBuildOutcome;
use crate::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_embedding_processor::ContinuousBatchEmbeddingProcessor;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_request_state::ContinuousBatchRequestState;
use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::decoded_image::DecodedImage;
use crate::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::grammar_sampler::GrammarSampler;
use crate::prepare_conversation_history_request::prepare_conversation_history_request;
use crate::prepared_conversation_history_request::PreparedConversationHistoryRequest;
use crate::resolve_grammar::resolve_grammar;
use crate::sample_token_at_batch_index::sample_token_at_batch_index;
use crate::sampling_outcome::SamplingOutcome;
use crate::sequence_id_pool::SequenceIdPool;
use crate::slot_guard::SlotGuard;
use crate::tool_call_pipeline::ToolCallPipeline;
use crate::tool_call_validator::ToolCallValidator;
use crate::validator_build_error::ValidatorBuildError;

fn send_generated_token_result_or_warn(
    agent_name: Option<&str>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    result: GeneratedTokenResult,
) {
    if generated_tokens_tx.send(result).is_err() {
        warn!("{agent_name:?}: failed to send result to client (receiver dropped)");
    }
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
}

impl ContinuousBatchScheduler {
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "required for FFI lifetime extension with llama.cpp"
    )]
    pub fn new(
        command_rx: Receiver<ContinuousBatchSchedulerCommand>,
        scheduler_context: Arc<ContinuousBatchSchedulerContext>,
        llama_context: LlamaContext,
        max_concurrent_sequences: i32,
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
        ContinueFromConversationHistoryRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params,
            slot_guard,
        }: ContinueFromConversationHistoryRequest,
    ) {
        let prepared = match prepare_conversation_history_request(
            params,
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
                parse_tool_calls,
                tools,
            } => {
                if let Err(err) = self.accept_text_prompt(
                    &raw_prompt,
                    max_tokens,
                    grammar_sampler,
                    parse_tool_calls,
                    tools,
                    generated_tokens_tx,
                    generate_tokens_stop_rx,
                    slot_guard,
                ) {
                    error!(
                        "{:?}: failed to accept text prompt: {err:#}",
                        self.scheduler_context.agent_name
                    );
                }
            }
            PreparedConversationHistoryRequest::MultimodalPrompt {
                raw_prompt,
                images,
                max_tokens,
                grammar_sampler,
                parse_tool_calls,
                tools,
            } => {
                let multimodal_context = self.scheduler_context.multimodal_context.clone();

                if let Some(multimodal_context) = multimodal_context.as_ref()
                    && let Err(err) = self.accept_multimodal_request(
                        multimodal_context,
                        raw_prompt,
                        &images,
                        max_tokens,
                        grammar_sampler,
                        parse_tool_calls,
                        tools,
                        generated_tokens_tx,
                        generate_tokens_stop_rx,
                        slot_guard,
                    )
                {
                    error!(
                        "{:?}: failed to accept multimodal request: {err:#}",
                        self.scheduler_context.agent_name
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
            slot_guard,
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

        if let Err(err) = self.accept_text_prompt(
            &raw_prompt,
            max_tokens,
            grammar_sampler,
            false,
            Vec::new(),
            generated_tokens_tx,
            generate_tokens_stop_rx,
            slot_guard,
        ) {
            error!(
                "{:?}: failed to accept raw prompt: {err:#}",
                self.scheduler_context.agent_name
            );
        }
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

                    send_generated_token_result_or_warn(
                        self.scheduler_context.agent_name.as_deref(),
                        generated_tokens_tx,
                        GeneratedTokenResult::GrammarInitializationFailed(message.clone()),
                    );

                    Err(anyhow!(message))
                }
            },
        )
    }

    #[expect(
        unsafe_code,
        reason = "the SchedulerContext owns the LlamaModel for the lifetime of the active_requests vec — same pattern as LlamaContext<'static> above"
    )]
    fn build_token_classifier_for_active_request(
        &self,
    ) -> llama_cpp_bindings::SampledTokenClassifier<'static> {
        let classifier = self.scheduler_context.model.sampled_token_classifier();

        unsafe {
            std::mem::transmute::<
                llama_cpp_bindings::SampledTokenClassifier<'_>,
                llama_cpp_bindings::SampledTokenClassifier<'static>,
            >(classifier)
        }
    }

    fn build_tool_call_pipeline(
        &self,
        tools: Vec<Tool<ValidatedParametersSchema>>,
        parse_tool_calls: bool,
    ) -> Result<ToolCallPipelineBuildOutcome> {
        if !parse_tool_calls || tools.is_empty() {
            return Ok(ToolCallPipelineBuildOutcome::Disabled);
        }

        let validator = match ToolCallValidator::from_tools(&tools) {
            Ok(validator) => validator,
            Err(ValidatorBuildError::InvalidSchema { tool_name, message }) => {
                return Ok(ToolCallPipelineBuildOutcome::SchemaInvalid(format!(
                    "tool {tool_name:?} parameters are not a valid JSON Schema: {message}"
                )));
            }
            Err(err @ ValidatorBuildError::SerializationFailed { .. }) => {
                return Err(anyhow::Error::from(err))
                    .context("failed to serialize tool parameters during validator build");
            }
        };

        let tools_json: Vec<serde_json::Value> = tools
            .into_iter()
            .map(|tool| serde_json::to_value(&tool))
            .collect::<Result<Vec<_>, _>>()
            .context("failed to serialize tools to JSON")?;

        let pipeline =
            ToolCallPipeline::new(self.scheduler_context.model.clone(), &tools_json, validator)
                .context("failed to serialize tools for tool-call pipeline")?;

        Ok(ToolCallPipelineBuildOutcome::Ready(pipeline))
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "these are distinct concerns (the prompt, the generation config, the output channel, the stop signal, the slot guard) that do not form a cohesive value object; bundling them would violate single-responsibility grouping"
    )]
    fn accept_text_prompt(
        &mut self,
        prompt: &str,
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
        parse_tool_calls: bool,
        tools: Vec<Tool<ValidatedParametersSchema>>,
        generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
        generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
        slot_guard: SlotGuard,
    ) -> Result<()> {
        let tool_call_pipeline = match self
            .build_tool_call_pipeline(tools, parse_tool_calls)
            .context("failed to build tool-call pipeline for text prompt")?
        {
            ToolCallPipelineBuildOutcome::Disabled => None,
            ToolCallPipelineBuildOutcome::Ready(pipeline) => Some(pipeline),
            ToolCallPipelineBuildOutcome::SchemaInvalid(message) => {
                error!(
                    "{:?}: rejecting text prompt: {message}",
                    self.scheduler_context.agent_name
                );

                send_generated_token_result_or_warn(
                    self.scheduler_context.agent_name.as_deref(),
                    &generated_tokens_tx,
                    GeneratedTokenResult::ToolSchemaInvalid(message),
                );

                return Ok(());
            }
        };

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

            send_generated_token_result_or_warn(
                self.scheduler_context.agent_name.as_deref(),
                &generated_tokens_tx,
                GeneratedTokenResult::SamplerError(message),
            );

            return Ok(());
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

                send_generated_token_result_or_warn(
                    self.scheduler_context.agent_name.as_deref(),
                    &generated_tokens_tx,
                    GeneratedTokenResult::SamplerError(message),
                );

                return Ok(());
            }
        };

        let Ok(llama_grammar_sampler) =
            self.create_grammar_llama_sampler(grammar_sampler, &generated_tokens_tx)
        else {
            self.sequence_id_pool.release(sequence_id);

            return Ok(());
        };

        let chain = self.create_sampler_chain();

        let mut token_classifier = self.build_token_classifier_for_active_request();

        token_classifier.record_prompt_tokens(prompt_tokens.len() as u64);
        token_classifier.ingest_prompt_tokens(&prompt_tokens);

        self.clear_kv_cache_for_sequence(sequence_id);

        debug!(
            "{:?}: accepted text prompt request on sequence {sequence_id} ({} tokens)",
            self.scheduler_context.agent_name,
            prompt_tokens.len()
        );

        self.active_requests.push(ContinuousBatchActiveRequest {
            state: ContinuousBatchRequestState {
                current_token_position: 0,
                i_batch: None,
                max_tokens,
                pending_sampled_token: None,
                phase: ContinuousBatchRequestPhase::Ingesting,
                prompt_tokens,
                prompt_tokens_ingested: 0,
                sequence_id,
            },
            chain,
            token_classifier,
            grammar_sampler: llama_grammar_sampler,
            generated_tokens_tx,
            generate_tokens_stop_rx,
            slot_guard,
            tool_call_pipeline,
        });

        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "these are distinct concerns (the multimodal context, prompt, images, generation config, the output channel, the stop signal, the slot guard) that do not form a cohesive value object; bundling them would violate single-responsibility grouping"
    )]
    fn accept_multimodal_request(
        &mut self,
        multimodal_context: &MtmdContext,
        prompt: String,
        images: &[DecodedImage],
        max_tokens: i32,
        grammar_sampler: Option<GrammarSampler>,
        parse_tool_calls: bool,
        tools: Vec<Tool<ValidatedParametersSchema>>,
        generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
        generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
        slot_guard: SlotGuard,
    ) -> Result<()> {
        let tool_call_pipeline = match self
            .build_tool_call_pipeline(tools, parse_tool_calls)
            .context("failed to build tool-call pipeline for multimodal request")?
        {
            ToolCallPipelineBuildOutcome::Disabled => None,
            ToolCallPipelineBuildOutcome::Ready(pipeline) => Some(pipeline),
            ToolCallPipelineBuildOutcome::SchemaInvalid(message) => {
                error!(
                    "{:?}: rejecting multimodal request: {message}",
                    self.scheduler_context.agent_name
                );

                send_generated_token_result_or_warn(
                    self.scheduler_context.agent_name.as_deref(),
                    &generated_tokens_tx,
                    GeneratedTokenResult::ToolSchemaInvalid(message),
                );

                return Ok(());
            }
        };

        let Some(sequence_id) = self.sequence_id_pool.acquire() else {
            let message = format!(
                "{:?}: no available sequence slots for multimodal request",
                self.scheduler_context.agent_name
            );

            error!("{message}");

            send_generated_token_result_or_warn(
                self.scheduler_context.agent_name.as_deref(),
                &generated_tokens_tx,
                GeneratedTokenResult::SamplerError(message),
            );

            return Ok(());
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

                send_generated_token_result_or_warn(
                    self.scheduler_context.agent_name.as_deref(),
                    &generated_tokens_tx,
                    GeneratedTokenResult::ImageDecodingFailed(message),
                );

                return Ok(());
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

                send_generated_token_result_or_warn(
                    self.scheduler_context.agent_name.as_deref(),
                    &generated_tokens_tx,
                    GeneratedTokenResult::SamplerError(message),
                );

                return Ok(());
            }
        };

        let batch_size = self.scheduler_context.inference_parameters.n_batch;

        self.clear_kv_cache_for_sequence(sequence_id);

        self.harvest_pending_samples_before_external_decode();

        let mut token_classifier = self.build_token_classifier_for_active_request();

        let batch_size_i32 = i32::try_from(batch_size).context("batch_size does not fit in i32")?;

        let eval_outcome = token_classifier.eval_multimodal_chunks(
            &input_chunks,
            multimodal_context,
            &self.llama_context,
            0,
            sequence_id,
            batch_size_i32,
            true,
        );

        let tokens_ingested = match eval_outcome {
            Ok(tokens_ingested) => tokens_ingested,
            Err(EvalMultimodalChunksError::EvalFailed(
                MtmdEvalError::ImageChunkExceedsBatchSize(mismatch),
            )) => {
                warn!(
                    "{:?}: refused multimodal request: image chunk has {} tokens but n_batch is {}",
                    self.scheduler_context.agent_name, mismatch.image_tokens, mismatch.n_batch,
                );

                self.sequence_id_pool.release(sequence_id);

                send_generated_token_result_or_warn(
                    self.scheduler_context.agent_name.as_deref(),
                    &generated_tokens_tx,
                    GeneratedTokenResult::ImageExceedsBatchSize(OversizedImageDetails {
                        image_tokens: mismatch.image_tokens,
                        n_batch: mismatch.n_batch,
                    }),
                );

                return Ok(());
            }
            Err(err) => {
                let message = format!(
                    "{:?}: failed to ingest multimodal prompt: {err}",
                    self.scheduler_context.agent_name
                );

                error!("{message}");
                self.sequence_id_pool.release(sequence_id);

                send_generated_token_result_or_warn(
                    self.scheduler_context.agent_name.as_deref(),
                    &generated_tokens_tx,
                    GeneratedTokenResult::SamplerError(message),
                );

                return Ok(());
            }
        };

        self.llama_context.mark_logits_initialized(-1);

        let Ok(llama_grammar_sampler) =
            self.create_grammar_llama_sampler(grammar_sampler, &generated_tokens_tx)
        else {
            self.sequence_id_pool.release(sequence_id);

            return Ok(());
        };

        let chain = self.create_sampler_chain();

        debug!(
            "{:?}: accepted multimodal request on sequence {sequence_id} ({tokens_ingested} tokens ingested)",
            self.scheduler_context.agent_name
        );

        self.active_requests.push(ContinuousBatchActiveRequest {
            state: ContinuousBatchRequestState {
                current_token_position: tokens_ingested,
                i_batch: Some(-1),
                max_tokens,
                pending_sampled_token: None,
                phase: ContinuousBatchRequestPhase::Generating,
                prompt_tokens: Vec::new(),
                prompt_tokens_ingested: 0,
                sequence_id,
            },
            chain,
            token_classifier,
            grammar_sampler: llama_grammar_sampler,
            generated_tokens_tx,
            generate_tokens_stop_rx,
            slot_guard,
            tool_call_pipeline,
        });

        Ok(())
    }

    fn harvest_pending_samples_before_external_decode(&mut self) {
        for active_request in &mut self.active_requests {
            if !matches!(
                active_request.state.phase,
                ContinuousBatchRequestPhase::Generating
            ) {
                continue;
            }

            if active_request.state.pending_sampled_token.is_some() {
                continue;
            }

            let Some(batch_index) = active_request.state.i_batch else {
                continue;
            };

            match sample_token_at_batch_index(
                &self.llama_context,
                batch_index,
                &mut active_request.chain,
                &mut active_request.grammar_sampler,
            ) {
                Ok(SamplingOutcome::Token(raw_token)) => {
                    // Update classifier state (section / usage counters) but drop the
                    // outcomes — harvest-sampled tokens are funnelled into the next
                    // batch via `pending_sampled_token`; their user-visible emission
                    // happens in `advance_generating_phase` after the next decode,
                    // not here.
                    let _ = active_request.token_classifier.ingest(raw_token);
                    active_request.state.pending_sampled_token =
                        Some(llama_cpp_bindings::SampledToken::Content(raw_token));
                    active_request.state.i_batch = None;
                }
                Ok(SamplingOutcome::AllCandidatesEliminated) => {
                    error!(
                        "{:?}: sequence {} pre-eval harvest exhausted candidates",
                        self.scheduler_context.agent_name, active_request.state.sequence_id
                    );
                    active_request.complete_with_outcome(
                        self.scheduler_context.agent_name.as_deref(),
                        GeneratedTokenResult::SamplerError(
                            "all token candidates were eliminated during sampling".to_owned(),
                        ),
                    );
                }
                Ok(SamplingOutcome::GrammarRejectedModelOutput(message)) => {
                    error!(
                        "{:?}: sequence {} pre-eval harvest grammar rejected: {message}",
                        self.scheduler_context.agent_name, active_request.state.sequence_id
                    );
                    active_request.complete_with_outcome(
                        self.scheduler_context.agent_name.as_deref(),
                        GeneratedTokenResult::GrammarRejectedModelOutput(message),
                    );
                }
                Err(err) => {
                    error!(
                        "{:?}: sequence {} pre-eval harvest sampling error: {err:#}",
                        self.scheduler_context.agent_name, active_request.state.sequence_id
                    );
                    active_request.complete_with_outcome(
                        self.scheduler_context.agent_name.as_deref(),
                        GeneratedTokenResult::SamplerError(err.to_string()),
                    );
                }
            }
        }
    }

    fn check_stop_signals(&mut self) {
        for active_request in &mut self.active_requests {
            if active_request.is_stop_requested() {
                let summary = GenerationSummary {
                    usage: *active_request.token_classifier.usage(),
                };

                active_request.complete_with_outcome(
                    self.scheduler_context.agent_name.as_deref(),
                    GeneratedTokenResult::Done(summary),
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
            .any(|request| !matches!(request.state.phase, ContinuousBatchRequestPhase::Completed))
    }

    fn execute_one_iteration(&mut self) -> Result<()> {
        self.advance_generating_requests();

        let n_batch = self.scheduler_context.inference_parameters.n_batch;
        let assemble_phase = AssembleBatchPhase { n_batch };

        loop {
            let max_sequences = self.active_requests.len();

            let max_sequences_i32 = i32::try_from(max_sequences.max(1))
                .context("max sequence count does not fit in i32")?;
            let mut pass = BatchPass::new(n_batch, max_sequences_i32)?;

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
                ContinuousBatchRequestPhase::Completed
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
                evicted_request.state.sequence_id,
                evicted_request.state.current_token_position
            );

            send_generated_token_result_or_warn(
                self.scheduler_context.agent_name.as_deref(),
                &evicted_request.generated_tokens_tx,
                GeneratedTokenResult::SamplerError(
                    "Request evicted due to KV cache pressure".to_owned(),
                ),
            );

            evicted_request.state.phase = ContinuousBatchRequestPhase::Completed;

            self.cleanup_completed_request(eviction_index);
        }
    }

    fn remove_completed_requests(&mut self) {
        let mut removal_index = 0;

        while removal_index < self.active_requests.len() {
            if matches!(
                self.active_requests[removal_index].state.phase,
                ContinuousBatchRequestPhase::Completed
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
        let removed_request = self.active_requests.swap_remove(index);

        self.clear_kv_cache_for_sequence(removed_request.state.sequence_id);

        self.sequence_id_pool
            .release(removed_request.state.sequence_id);

        let usage = removed_request.token_classifier.usage();

        debug!(
            "{:?}: cleaned up sequence {} ({} completion tokens generated)",
            self.scheduler_context.agent_name,
            removed_request.state.sequence_id,
            usage.content_tokens + usage.reasoning_tokens + usage.undeterminable_tokens,
        );
    }
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;
    use tokio::sync::mpsc;

    use super::send_generated_token_result_or_warn;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    #[test]
    fn delivers_result_to_a_live_receiver() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        send_generated_token_result_or_warn(
            Some("agent"),
            &generated_tokens_tx,
            GeneratedTokenResult::SamplerError("boom".to_owned()),
        );

        assert!(matches!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::SamplerError(message)) if message == "boom"
        ));
    }

    #[test]
    fn warns_without_panicking_when_the_receiver_was_dropped() {
        log::set_max_level(LevelFilter::Trace);

        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        send_generated_token_result_or_warn(
            None,
            &generated_tokens_tx,
            GeneratedTokenResult::SamplerError("boom".to_owned()),
        );

        assert!(generated_tokens_tx.is_closed());
    }
}
