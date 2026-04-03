use core::num::NonZeroU32;
use std::cmp::max;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::thread::available_parallelism;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdContextParams;
use llama_cpp_bindings_sys::LLAMA_FLASH_ATTN_TYPE_ENABLED;
use log::error;
use log::info;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ChatTemplateDoesNotCompileParams;
use paddler_types::agent_issue_params::ModelPath;
use paddler_types::chat_template::ChatTemplate;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::model_metadata::ModelMetadata;
use tokio::sync::oneshot;

use crate::agent::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;
use crate::agent::continuous_batch_scheduler::ContinuousBatchScheduler;
use crate::agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::agent::model_metadata_holder::ModelMetadataHolder;
use crate::agent_issue_fix::AgentIssueFix;
use crate::chat_template_renderer::ChatTemplateRenderer;
use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType;
use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

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
    pub async fn spawn(&self) -> Result<ContinuousBatchArbiterHandle> {
        let (chat_template_loaded_tx, chat_template_loaded_rx) = oneshot::channel::<()>();
        let (model_loaded_tx, model_loaded_rx) = oneshot::channel::<()>();

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

            #[expect(
                clippy::cast_sign_loss,
                reason = "desired_slots_total is always positive"
            )]
            let n_seq_max = if inference_parameters.enable_embeddings {
                inference_parameters.embedding_n_seq_max
            } else {
                desired_slots_total as u32
            };

            let context_params = LlamaContextParams::default()
                .with_embeddings(inference_parameters.enable_embeddings)
                .with_n_ctx(NonZeroU32::new(inference_parameters.context_size))
                .with_flash_attention_policy(LLAMA_FLASH_ATTN_TYPE_ENABLED)
                .with_n_seq_max(n_seq_max)
                .with_n_threads(n_threads)
                .with_n_threads_batch(n_threads_batch)
                .with_pooling_type(
                    inference_parameters
                        .pooling_type
                        .clone()
                        .to_llama_pooling_type(),
                );

            let model = Arc::new(
                LlamaModel::load_from_file(&llama_backend, model_path.clone(), &{
                    if cfg!(any(
                        feature = "cuda",
                        feature = "vulkan",
                        target_os = "macos"
                    )) {
                        LlamaModelParams::default().with_n_gpu_layers(1000)
                    } else {
                        LlamaModelParams::default()
                    }
                })
                .context("Unable to load model from file")?,
            );

            if model_loaded_tx.send(()).is_err() {
                let message = format!(
                    "Failed to send model loaded signal for model at path: {}",
                    model_path.display()
                );

                error!("{message}");

                return Err(anyhow!(message));
            }

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

            if chat_template_loaded_tx.send(()).is_err() {
                let message = format!(
                    "Failed to send chat template loaded signal for model at path: {}",
                    model_path.display()
                );

                error!("{message}");

                return Err(anyhow!(message));
            }

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
                inference_parameters,
                model_path: model_path.clone(),
                multimodal_context,
                token_bos_str: model.token_to_piece(
                    model.token_bos(),
                    &mut special_token_decoder,
                    true,
                    None,
                )?,
                token_nl_str: model.token_to_piece(
                    model.token_nl(),
                    &mut special_token_decoder,
                    true,
                    None,
                )?,
                token_eos_str: model.token_to_piece(
                    model.token_eos(),
                    &mut special_token_decoder,
                    true,
                    None,
                )?,
                model: model.clone(),
            });

            let llama_context = model.new_context(&llama_backend, context_params)?;

            for slot_index in 0..desired_slots_total {
                slot_aggregated_status_manager
                    .slot_aggregated_status
                    .increment_total_slots();

                #[expect(clippy::cast_sign_loss, reason = "slot_index is always non-negative")]
                slot_aggregated_status_manager
                    .slot_aggregated_status
                    .register_fix(&AgentIssueFix::SlotStarted(slot_index as u32));
            }

            let mut scheduler = ContinuousBatchScheduler::new(
                command_rx,
                scheduler_context,
                llama_context,
                desired_slots_total,
                slot_aggregated_status_manager
                    .slot_aggregated_status
                    .clone(),
            );

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

        Ok(ContinuousBatchArbiterHandle {
            command_tx,
            scheduler_thread_handle: Some(scheduler_thread_handle),
        })
    }
}
