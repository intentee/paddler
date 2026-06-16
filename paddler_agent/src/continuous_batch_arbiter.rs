use core::num::NonZeroU32;
use std::cmp::max;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::thread::available_parallelism;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdContextParams;
use llama_cpp_bindings_sys::LLAMA_FLASH_ATTN_TYPE_AUTO;
use log::error;
use log::info;
use log::warn;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::model_metadata::ModelMetadata;
use tokio::sync::oneshot;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_issue_fix::AgentIssueFix;
use crate::agent_kv_cache_dtype::AgentKvCacheDtype;
use crate::agent_pooling_type::AgentPoolingType;
use crate::chat_template_renderer::ChatTemplateRenderer;
use crate::continuous_batch_arbiter_build_outcome::ContinuousBatchArbiterBuildOutcome;
use crate::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;
use crate::continuous_batch_scheduler::ContinuousBatchScheduler;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::converts_to_llama_kv_cache_dtype::ConvertsToLlamaKvCacheDtype;
use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType;
use crate::model_metadata_holder::ModelMetadataHolder;
use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

fn send_startup_signal_or_fail(
    signal_tx: oneshot::Sender<()>,
    failure_message: String,
) -> Result<()> {
    if signal_tx.send(()).is_err() {
        error!("{failure_message}");

        return Err(anyhow!(failure_message));
    }

    Ok(())
}

pub struct ContinuousBatchArbiter {
    pub agent_name: Option<String>,
    pub chat_template_override: Option<ChatTemplate>,
    pub desired_slots_total: i32,
    pub inference_parameters: InferenceParameters,
    pub multimodal_projection_path: Option<PathBuf>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub model_path: PathBuf,
    pub model_path_string: String,
    pub slot_aggregated_status_manager: Arc<SlotAggregatedStatusManager>,
}

impl ContinuousBatchArbiter {
    #[must_use]
    pub fn build_from_applicable_state(
        applicable_state: AgentApplicableState,
        agent_name: Option<String>,
        desired_slots_total: i32,
        model_metadata_holder: Arc<ModelMetadataHolder>,
        slot_aggregated_status_manager: Arc<SlotAggregatedStatusManager>,
    ) -> ContinuousBatchArbiterBuildOutcome {
        let Some(model_path) = applicable_state.model_path else {
            return ContinuousBatchArbiterBuildOutcome::NoModelConfigured;
        };

        let model_path_string = model_path.display().to_string();

        ContinuousBatchArbiterBuildOutcome::ReadyToSpawn(Box::new(Self {
            agent_name,
            chat_template_override: applicable_state.chat_template_override,
            desired_slots_total,
            inference_parameters: applicable_state.inference_parameters,
            multimodal_projection_path: applicable_state.multimodal_projection_path,
            model_metadata_holder,
            model_path,
            model_path_string,
            slot_aggregated_status_manager,
        }))
    }

    pub async fn spawn(&self) -> Result<ContinuousBatchArbiterHandle> {
        let (chat_template_loaded_tx, chat_template_loaded_rx) = oneshot::channel::<()>();
        let (model_loaded_tx, model_loaded_rx) = oneshot::channel::<()>();
        let (agent_warm_and_scheduler_running_tx, agent_warm_and_scheduler_running_rx) =
            oneshot::channel::<()>();

        let available_parallelism_value: i32 = available_parallelism()?.get().try_into()?;
        let n_threads = max(2, available_parallelism_value / 2);
        let n_threads_batch = max(2, available_parallelism_value / 2);

        info!("Using threads for parallelism threads/batch: {n_threads}/{n_threads_batch}");

        let (command_tx, command_rx) = std::sync::mpsc::channel();

        let agent_name_clone = self.agent_name.clone();
        let desired_slots_total = self.desired_slots_total;
        let inference_parameters = self.inference_parameters.clone();
        let model_metadata_holder = self.model_metadata_holder.clone();
        let multimodal_projection_path = self.multimodal_projection_path.clone();
        let model_path = self.model_path.clone();
        let model_path_string_clone = self.model_path_string.clone();
        let model_path_string = self.model_path_string.clone();
        let chat_template_override = self.chat_template_override.clone();
        let slot_aggregated_status_manager = self.slot_aggregated_status_manager.clone();

        let scheduler_thread_handle = thread::spawn(move || -> Result<()> {
            let llama_backend =
                Arc::new(LlamaBackend::init().context("Unable to initialize llama.cpp backend")?);

            let n_seq_max = u32::try_from(desired_slots_total)
                .context("desired_slots_total does not fit in u32")?;

            let inference_parameters_n_batch_u32 = u32::try_from(inference_parameters.n_batch)
                .context("n_batch does not fit in u32")?;

            let context_params = LlamaContextParams::default()
                .with_embeddings(inference_parameters.enable_embeddings)
                .with_n_ctx(NonZeroU32::new(inference_parameters.context_size))
                .with_n_batch(inference_parameters_n_batch_u32)
                .with_flash_attention_policy(LLAMA_FLASH_ATTN_TYPE_AUTO)
                .with_n_seq_max(n_seq_max)
                .with_n_threads(n_threads)
                .with_n_threads_batch(n_threads_batch)
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

            send_startup_signal_or_fail(
                model_loaded_tx,
                format!(
                    "Failed to send model loaded signal for model at path: {}",
                    model_path.display()
                ),
            )?;

            let mut model_metadata = ModelMetadata::default();

            for metadata_index in 0..model.meta_count() {
                model_metadata.set_meta_field(
                    model.meta_key_by_index(metadata_index)?,
                    model.meta_val_str_by_index(metadata_index)?,
                );
            }

            model_metadata_holder.set_model_metadata(model_metadata);

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

            send_startup_signal_or_fail(
                chat_template_loaded_tx,
                format!(
                    "Failed to send chat template loaded signal for model at path: {}",
                    model_path.display()
                ),
            )?;

            let chat_template_renderer = Arc::new(
                match ChatTemplateRenderer::new(ChatTemplate {
                    content: llama_chat_template_string.clone(),
                })
                .context("Failed to create chat template renderer")
                {
                    Ok(renderer) => {
                        slot_aggregated_status_manager
                            .slot_aggregated_status
                            .register_fix(&AgentIssueFix::ChatTemplateIsCompiled(ModelPath {
                                model_path: model_path.display().to_string(),
                            }));

                        renderer
                    }
                    Err(err) => {
                        slot_aggregated_status_manager
                            .slot_aggregated_status
                            .register_issue(AgentIssue::ChatTemplateDoesNotCompile(
                                ChatTemplateDoesNotCompileParams {
                                    error: format!("{err}"),
                                    model_path: ModelPath {
                                        model_path: model_path.display().to_string(),
                                    },
                                    template_content: llama_chat_template_string,
                                },
                            ));

                        return Err(err);
                    }
                },
            );

            slot_aggregated_status_manager
                .slot_aggregated_status
                .set_model_path(Some(model_path_string_clone));

            let multimodal_context = match multimodal_projection_path {
                Some(multimodal_projection_path) => {
                    let multimodal_projection_path_str =
                        multimodal_projection_path.to_string_lossy();

                    match MtmdContext::init_from_file(
                        &multimodal_projection_path_str,
                        &model,
                        &MtmdContextParams::default(),
                    ) {
                        Ok(mtmd_context) => {
                            slot_aggregated_status_manager
                                .slot_aggregated_status
                                .register_fix(&AgentIssueFix::MultimodalProjectionIsLoaded(
                                    ModelPath {
                                        model_path: multimodal_projection_path
                                            .display()
                                            .to_string(),
                                    },
                                ));

                            info!(
                                "Multimodal context initialized from: {}",
                                multimodal_projection_path.display()
                            );

                            Some(Arc::new(mtmd_context))
                        }
                        Err(err) => {
                            slot_aggregated_status_manager
                                .slot_aggregated_status
                                .register_issue(AgentIssue::MultimodalProjectionCannotBeLoaded(
                                    ModelPath {
                                        model_path: multimodal_projection_path
                                            .display()
                                            .to_string(),
                                    },
                                ));

                            return Err(err.into());
                        }
                    }
                }
                None => None,
            };

            let mut special_token_decoder = encoding_rs::UTF_8.new_decoder();

            let scheduler_context = Arc::new(ContinuousBatchSchedulerContext {
                agent_name: agent_name_clone,
                chat_template_renderer,
                desired_slots_total,
                inference_parameters,
                model_path: model_path.clone(),
                multimodal_context,
                token_bos_str: model.token_to_piece(
                    &SampledToken::Content(model.token_bos()),
                    &mut special_token_decoder,
                    true,
                    None,
                )?,
                token_nl_str: model.token_to_piece(
                    &SampledToken::Content(model.token_nl()),
                    &mut special_token_decoder,
                    true,
                    None,
                )?,
                token_eos_str: model.token_to_piece(
                    &SampledToken::Content(model.token_eos()),
                    &mut special_token_decoder,
                    true,
                    None,
                )?,
                model: model.clone(),
            });

            let mut llama_context =
                match LlamaContext::from_model(&model, &llama_backend, context_params)
                    .context("Unable to create llama.cpp context")
                {
                    Ok(context) => context,
                    Err(err) => {
                        for slot_index in 0..n_seq_max {
                            slot_aggregated_status_manager
                                .slot_aggregated_status
                                .register_issue(AgentIssue::SlotCannotStart(
                                    SlotCannotStartParams {
                                        error: format!("{err:#}"),
                                        slot_index,
                                    },
                                ));
                        }

                        return Err(err);
                    }
                };

            Self::run_warmup_decode(
                &model,
                &mut llama_context,
                scheduler_context.inference_parameters.n_batch,
                desired_slots_total,
            );

            let mut scheduler = ContinuousBatchScheduler::new(
                command_rx,
                scheduler_context,
                llama_context,
                desired_slots_total,
            );

            send_startup_signal_or_fail(
                agent_warm_and_scheduler_running_tx,
                "Arbiter dropped the agent-warm-and-scheduler-running receiver before the scheduler could start".to_owned(),
            )?;

            scheduler.run();

            Ok(())
        });

        match model_loaded_rx
            .await
            .context("Failed to receive model loaded signal")
        {
            Ok(()) => {
                self.slot_aggregated_status_manager
                    .slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelIsLoaded(ModelPath {
                        model_path: model_path_string.clone(),
                    }));
            }
            Err(err) => {
                error!("Failed to load model: {err}");

                self.slot_aggregated_status_manager
                    .slot_aggregated_status
                    .register_issue(AgentIssue::ModelCannotBeLoaded(ModelPath {
                        model_path: model_path_string.clone(),
                    }));
            }
        }

        match chat_template_loaded_rx
            .await
            .context("Failed to receive chat template loaded signal")
        {
            Ok(()) => {
                self.slot_aggregated_status_manager
                    .slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelChatTemplateIsLoaded(ModelPath {
                        model_path: model_path_string.clone(),
                    }));
            }
            Err(err) => {
                error!("Failed to load chat template: {err}");

                if !self
                    .slot_aggregated_status_manager
                    .slot_aggregated_status
                    .has_issue(&AgentIssue::ModelCannotBeLoaded(ModelPath {
                        model_path: model_path_string.clone(),
                    }))
                {
                    self.slot_aggregated_status_manager
                        .slot_aggregated_status
                        .register_issue(AgentIssue::UnableToFindChatTemplate(ModelPath {
                            model_path: model_path_string.clone(),
                        }));
                }
            }
        }

        agent_warm_and_scheduler_running_rx.await.context(
            "Scheduler thread did not signal agent-warm-and-scheduler-running before exiting",
        )?;

        let desired_slots_total_u32 = u32::try_from(self.desired_slots_total)
            .context("desired_slots_total does not fit in u32")?;

        for slot_index in 0..desired_slots_total_u32 {
            self.slot_aggregated_status_manager
                .slot_aggregated_status
                .increment_total_slots();

            self.slot_aggregated_status_manager
                .slot_aggregated_status
                .register_fix(&AgentIssueFix::SlotStarted(slot_index));
        }

        Ok(ContinuousBatchArbiterHandle {
            command_tx,
            scheduler_thread_handle,
        })
    }

    fn run_warmup_decode(
        model: &LlamaModel,
        llama_context: &mut LlamaContext<'_>,
        n_batch: usize,
        desired_slots_total: i32,
    ) {
        let warmup_tokens = vec![model.token_bos(); 4];
        let mut warmup_batch = match LlamaBatch::new(n_batch, desired_slots_total) {
            Ok(warmup_batch) => warmup_batch,
            Err(err) => {
                warn!("Warmup batch allocation failed: {err:#}");
                return;
            }
        };

        for sequence_index in 0..desired_slots_total {
            if let Err(err) = warmup_batch.add_sequence(&warmup_tokens, sequence_index, true) {
                warn!("Warmup batch add_sequence failed: {err:#}");
                return;
            }
        }

        llama_context.clear_kv_cache();
        if let Err(err) = llama_context.decode(&mut warmup_batch) {
            warn!("Warmup decode failed: {err:#}");
        }
        llama_context.synchronize();
        llama_context.clear_kv_cache();
    }
}
