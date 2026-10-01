use std::cmp::max;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use std::thread::available_parallelism;

use anyhow::Context as _;
use anyhow::Result;
use encoding_rs::Decoder;
use encoding_rs::UTF_8;
use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings::error::TokenToStringError;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdContextParams;
use llama_cpp_bindings::mtmd::micro_batch_tokens;
use llama_cpp_bindings::mtmd::mtmd_default_marker;
use llama_cpp_bindings::token::LlamaToken;
use llama_cpp_bindings_sys::LLAMA_FLASH_ATTN_TYPE_AUTO;
use log::debug;
use log::error;
use log::info;
use log::warn;
use tokio::sync::oneshot;
use tokio::task::spawn_blocking;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::media_marker::MediaMarker;
use paddler_messaging::model_metadata::ModelMetadata;

use crate::agent_kv_cache_dtype::AgentKvCacheDtype;
use crate::agent_pooling_type::AgentPoolingType;
use crate::chat_prompt_renderer::ChatPromptRenderer;
use crate::chat_template_load_status::ChatTemplateLoadStatus;
use crate::chat_template_renderer::ChatTemplateRenderer;
use crate::continuous_batch_arbiter_context::ContinuousBatchArbiterContext;
use crate::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;
use crate::continuous_batch_arbiter_spawn_outcome::ContinuousBatchArbiterSpawnOutcome;
use crate::continuous_batch_request_preparer::ContinuousBatchRequestPreparer;
use crate::continuous_batch_scheduler::ContinuousBatchScheduler;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use crate::converts_to_llama_kv_cache_dtype::ConvertsToLlamaKvCacheDtype;
use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType;
use crate::embedding_batch_preparer::EmbeddingBatchPreparer;
use crate::generation_request_preparer::GenerationRequestPreparer;
use crate::image_input::ImageInput;
use crate::join_scheduler_thread::join_scheduler_thread;
use crate::multimodal_prompt_support::MultimodalPromptSupport;
use crate::prompt_tokenizer::PromptTokenizer;
use crate::sampler_chain_factory::SamplerChainFactory;
use crate::send_startup_signal::send_startup_signal;
use crate::startup_signal_delivery::StartupSignalDelivery;
use crate::token_generation::TokenGeneration;
use crate::token_generation_support::TokenGenerationSupport;

const LOGICAL_CORES_PER_PHYSICAL_CORE: i32 = 2;
const MINIMUM_THREAD_COUNT: i32 = 2;

fn special_token_text(
    model: &LlamaModel,
    token: LlamaToken,
    decoder: &mut Decoder,
) -> Result<String, TokenToStringError> {
    model.token_to_piece(&SampledToken::Content(token), decoder, true, None)
}

pub struct ContinuousBatchArbiter {
    pub chat_template_override: Option<ChatTemplate>,
    pub context: ContinuousBatchArbiterContext,
    pub inference_parameters: InferenceParameters,
    pub model_path: PathBuf,
    pub multimodal_projection_path: Option<PathBuf>,
}

impl ContinuousBatchArbiter {
    pub async fn spawn(
        &self,
        cancellation_token: &CancellationToken,
    ) -> Result<ContinuousBatchArbiterSpawnOutcome> {
        if cancellation_token.is_cancelled() {
            return Ok(ContinuousBatchArbiterSpawnOutcome::Cancelled);
        }

        let desired_slots_total = self.context.slot_aggregated_status.desired_slots_total;
        let n_seq_max = u32::from(desired_slots_total);
        let (chat_template_loaded_tx, chat_template_loaded_rx) =
            oneshot::channel::<ChatTemplateLoadStatus>();
        let (model_loaded_tx, model_loaded_rx) = oneshot::channel::<()>();
        let (agent_warm_and_scheduler_running_tx, agent_warm_and_scheduler_running_rx) =
            oneshot::channel::<Arc<ContinuousBatchRequestPreparer>>();

        let available_parallelism_value: i32 = available_parallelism()?.get().try_into()?;
        let thread_count = max(
            MINIMUM_THREAD_COUNT,
            available_parallelism_value / LOGICAL_CORES_PER_PHYSICAL_CORE,
        );

        info!("Using {thread_count} threads for generation and for batch processing");

        let agent_name_clone = self.context.agent_name.clone();
        let inference_parameters = self.inference_parameters.clone();
        let model_metadata_holder = self.context.model_metadata_holder.clone();
        let multimodal_projection_path = self.multimodal_projection_path.clone();
        let model_path = self.model_path.clone();
        let model_issue_path = ModelPath::from(self.model_path.as_path());
        let chat_template_override = self.chat_template_override.clone();
        let slot_aggregated_status = self.context.slot_aggregated_status.clone();
        let agent_shutdown = cancellation_token.clone();

        let scheduler_thread_handle = thread::spawn(move || -> Result<()> {
            let llama_backend =
                Arc::new(LlamaBackend::init().context("Unable to initialize llama.cpp backend")?);

            let n_batch_tokens = inference_parameters.n_batch.tokens().get();

            let default_context_params = LlamaContextParams::default();
            let n_ubatch = if inference_parameters.enable_embeddings {
                n_batch_tokens
            } else {
                default_context_params.n_ubatch()
            };

            let context_params = default_context_params
                .with_embeddings(inference_parameters.enable_embeddings)
                .with_n_ctx(Some(inference_parameters.context_size))
                .with_n_batch(n_batch_tokens)
                .with_n_ubatch(n_ubatch)
                .with_flash_attention_policy(LLAMA_FLASH_ATTN_TYPE_AUTO)
                .with_n_seq_max(n_seq_max)
                .with_n_threads(thread_count)
                .with_n_threads_batch(thread_count)
                .with_pooling_type(
                    AgentPoolingType(inference_parameters.pooling_type.clone())
                        .to_llama_pooling_type(),
                )
                .with_type_k(
                    AgentKvCacheDtype(inference_parameters.k_cache_dtype.clone())
                        .to_llama_kv_cache_dtype(),
                )
                .with_type_v(
                    AgentKvCacheDtype(inference_parameters.v_cache_dtype.clone())
                        .to_llama_kv_cache_dtype(),
                );

            let model = Arc::new(
                LlamaModel::load_from_file(
                    &llama_backend,
                    model_path.clone(),
                    &LlamaModelParams::default()
                        .with_n_gpu_layers(inference_parameters.n_gpu_layers),
                )
                .context("Unable to load model from file")?,
            );

            if matches!(
                send_startup_signal(model_loaded_tx, ()),
                StartupSignalDelivery::Abandoned
            ) {
                return Ok(());
            }

            let mut model_metadata = ModelMetadata::default();

            for metadata_index in 0..model.meta_count() {
                model_metadata.set_meta_field(
                    model.meta_key_by_index(metadata_index)?,
                    model.meta_val_str_by_index(metadata_index)?,
                );
            }

            model_metadata_holder.set_model_metadata(model_metadata);

            let token_generation = if inference_parameters.enable_embeddings
                && chat_template_override.is_none()
            {
                if matches!(
                    send_startup_signal(
                        chat_template_loaded_tx,
                        ChatTemplateLoadStatus::SkippedForEmbeddings,
                    ),
                    StartupSignalDelivery::Abandoned
                ) {
                    return Ok(());
                }

                TokenGeneration::DisabledForEmbeddings
            } else {
                let llama_chat_template_string = match chat_template_override {
                    Some(chat_template) => chat_template.content,
                    None => model
                        .chat_template(None)
                        .context(format!(
                            "Failed to load chat template for model at path: {}",
                            model_path.display()
                        ))?
                        .to_string()?,
                };

                if matches!(
                    send_startup_signal(chat_template_loaded_tx, ChatTemplateLoadStatus::Loaded),
                    StartupSignalDelivery::Abandoned
                ) {
                    return Ok(());
                }

                let chat_template_renderer = match ChatTemplateRenderer::new(ChatTemplate {
                    content: llama_chat_template_string.clone(),
                })
                .context("Failed to create chat template renderer")
                {
                    Ok(renderer) => {
                        slot_aggregated_status.register_fix(
                            &AgentIssueFix::ChatTemplateIsCompiled(model_issue_path.clone()),
                        );

                        renderer
                    }
                    Err(err) => {
                        slot_aggregated_status.register_issue(
                            AgentIssue::ChatTemplateDoesNotCompile(
                                ChatTemplateDoesNotCompileParams {
                                    error: format!("{err}"),
                                    model_path: model_issue_path.clone(),
                                    template_content: llama_chat_template_string,
                                },
                            ),
                        );

                        return Err(err);
                    }
                };

                let mut special_token_decoder = UTF_8.new_decoder();

                TokenGeneration::Enabled(Box::new(TokenGenerationSupport {
                    chat_prompt_renderer: ChatPromptRenderer {
                        chat_template_renderer,
                        media_marker: MediaMarker::new(mtmd_default_marker()?.to_owned()),
                        token_bos_str: special_token_text(
                            &model,
                            model.token_bos(),
                            &mut special_token_decoder,
                        )?,
                        token_eos_str: special_token_text(
                            &model,
                            model.token_eos(),
                            &mut special_token_decoder,
                        )?,
                        token_nl_str: special_token_text(
                            &model,
                            model.token_nl(),
                            &mut special_token_decoder,
                        )?,
                    },
                    streaming_markers: model.streaming_markers()?,
                }))
            };

            slot_aggregated_status.set_model_path(Some(model_issue_path.model_path.clone()));

            let mut llama_context =
                match LlamaContext::from_model(&model, &llama_backend, context_params)
                    .context("Unable to create llama.cpp context")
                {
                    Ok(context) => context,
                    Err(err) => {
                        for slot_index in 0..n_seq_max {
                            slot_aggregated_status.register_issue(AgentIssue::SlotCannotStart(
                                SlotCannotStartParams {
                                    error: format!("{err:#}"),
                                    slot_index,
                                },
                            ));
                        }

                        return Err(err);
                    }
                };

            let sequence_context_size = llama_context.n_ctx_seq();

            let image_input = match multimodal_projection_path {
                Some(multimodal_projection_path) => {
                    let multimodal_projection_path_str =
                        multimodal_projection_path.to_string_lossy();

                    match MtmdContext::init_from_file(
                        &multimodal_projection_path_str,
                        &model,
                        &MtmdContextParams::default(),
                    ) {
                        Ok(mtmd_context) => {
                            slot_aggregated_status.register_fix(
                                &AgentIssueFix::MultimodalProjectionIsLoaded(ModelPath::from(
                                    multimodal_projection_path.as_path(),
                                )),
                            );

                            info!(
                                "Multimodal context initialized from: {}",
                                multimodal_projection_path.display()
                            );

                            ImageInput::Supported(MultimodalPromptSupport {
                                micro_batch_tokens: micro_batch_tokens(
                                    &llama_context,
                                    inference_parameters.n_batch.tokens(),
                                ),
                                multimodal_context: Arc::new(mtmd_context),
                                n_batch: inference_parameters.n_batch.tokens_i32(),
                                sequence_context_size,
                            })
                        }
                        Err(err) => {
                            slot_aggregated_status.register_issue(
                                AgentIssue::MultimodalProjectionCannotBeLoaded(ModelPath::from(
                                    multimodal_projection_path.as_path(),
                                )),
                            );

                            return Err(err.into());
                        }
                    }
                }
                None => ImageInput::Unsupported,
            };

            let (scheduler_command_tx, scheduler_command_rx) = channel();

            let request_preparer = Arc::new(ContinuousBatchRequestPreparer {
                agent_name: agent_name_clone.clone(),
                embedding_batch_preparer: EmbeddingBatchPreparer {
                    enable_embeddings: inference_parameters.enable_embeddings,
                    model: model.clone(),
                    n_batch: inference_parameters.n_batch.tokens_usize(),
                },
                generation_request_preparer: GenerationRequestPreparer {
                    image_input,
                    image_resize_to_fit: inference_parameters.image_resize_to_fit,
                    model: model.clone(),
                    prompt_tokenizer: PromptTokenizer {
                        model: model.clone(),
                        sequence_context_size,
                    },
                    sampler_chain_factory: SamplerChainFactory {
                        inference_parameters: inference_parameters.clone(),
                        n_vocab: model.n_vocab(),
                    },
                    token_generation,
                },
                llama_backend: llama_backend.clone(),
                preparation_tasks: TaskTracker::new(),
                scheduler_command_tx,
            });

            let mut batch = LlamaBatch::new(
                inference_parameters.n_batch.tokens_usize(),
                i32::from(desired_slots_total),
            )?;

            Self::run_warmup_decode(&model, &mut llama_context, &mut batch, desired_slots_total);

            let mut scheduler = ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
                agent_shutdown,
                batch,
                command_rx: scheduler_command_rx,
                llama_context,
                scheduler_context: ContinuousBatchSchedulerContext {
                    agent_name: agent_name_clone,
                    desired_slots_total,
                    inference_parameters,
                    model: model.clone(),
                },
            });

            if matches!(
                send_startup_signal(agent_warm_and_scheduler_running_tx, request_preparer),
                StartupSignalDelivery::Abandoned
            ) {
                return Ok(());
            }

            scheduler.run();

            Ok(())
        });

        let Some(startup_result) = cancellation_token
            .run_until_cancelled(self.await_startup_signals(
                model_loaded_rx,
                chat_template_loaded_rx,
                agent_warm_and_scheduler_running_rx,
            ))
            .await
        else {
            if let Err(err) = spawn_blocking(move || join_scheduler_thread(scheduler_thread_handle))
                .await
                .context("Failed to join the scheduler shutdown task")?
            {
                debug!("Scheduler thread ended while its spawn was being cancelled: {err}");
            }

            return Ok(ContinuousBatchArbiterSpawnOutcome::Cancelled);
        };

        let request_preparer = match startup_result {
            Ok(request_preparer) => request_preparer,
            Err(startup_error) => {
                return spawn_blocking(move || join_scheduler_thread(scheduler_thread_handle))
                    .await
                    .context("Failed to join the scheduler thread after its startup failed")?
                    .and(Err(startup_error));
            }
        };

        for slot_index in 0..n_seq_max {
            self.context.slot_aggregated_status.increment_total_slots();

            self.context
                .slot_aggregated_status
                .register_fix(&AgentIssueFix::SlotStarted(slot_index));
        }

        Ok(ContinuousBatchArbiterSpawnOutcome::Ready(
            ContinuousBatchArbiterHandle {
                request_preparer,
                scheduler_thread_handle,
            },
        ))
    }

    async fn await_startup_signals(
        &self,
        model_loaded_rx: oneshot::Receiver<()>,
        chat_template_loaded_rx: oneshot::Receiver<ChatTemplateLoadStatus>,
        agent_warm_and_scheduler_running_rx: oneshot::Receiver<Arc<ContinuousBatchRequestPreparer>>,
    ) -> Result<Arc<ContinuousBatchRequestPreparer>> {
        let model_issue_path = ModelPath::from(self.model_path.as_path());

        match model_loaded_rx
            .await
            .context("Failed to receive model loaded signal")
        {
            Ok(()) => {
                self.context
                    .slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelIsLoaded(model_issue_path.clone()));
            }
            Err(err) => {
                error!("Failed to load model: {err}");

                self.context
                    .slot_aggregated_status
                    .register_issue(AgentIssue::ModelCannotBeLoaded(model_issue_path.clone()));
            }
        }

        match chat_template_loaded_rx
            .await
            .context("Failed to receive chat template loaded signal")
        {
            Ok(ChatTemplateLoadStatus::Loaded) => {
                self.context.slot_aggregated_status.register_fix(
                    &AgentIssueFix::ModelChatTemplateIsLoaded(model_issue_path.clone()),
                );
            }
            Ok(ChatTemplateLoadStatus::SkippedForEmbeddings) => {}
            Err(err) => {
                error!("Failed to load chat template: {err}");

                if !self
                    .context
                    .slot_aggregated_status
                    .has_issue(&AgentIssue::ModelCannotBeLoaded(model_issue_path.clone()))
                {
                    self.context
                        .slot_aggregated_status
                        .register_issue(AgentIssue::UnableToFindChatTemplate(model_issue_path));
                }
            }
        }

        agent_warm_and_scheduler_running_rx.await.context(
            "Scheduler thread did not signal agent-warm-and-scheduler-running before exiting",
        )
    }

    fn run_warmup_decode(
        model: &LlamaModel,
        llama_context: &mut LlamaContext<'_>,
        warmup_batch: &mut LlamaBatch<'static>,
        desired_slots_total: u16,
    ) {
        let warmup_tokens = [model.token_bos(), model.token_eos()];

        for sequence_index in 0..i32::from(desired_slots_total) {
            if let Err(err) = warmup_batch.add_sequence(&warmup_tokens, sequence_index, true) {
                warn!("Warmup batch add_sequence failed: {err:#}");
                return;
            }
        }

        llama_context.clear_kv_cache();
        if let Err(err) = llama_context.decode(warmup_batch) {
            warn!("Warmup decode failed: {err:#}");
        }
        llama_context.synchronize();
        llama_context.clear_kv_cache();
    }
}
