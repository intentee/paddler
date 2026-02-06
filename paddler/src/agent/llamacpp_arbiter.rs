use core::num::NonZeroU32;
use std::cmp::max;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::available_parallelism;

use actix::System;
use actix::sync::SyncArbiter;
use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_sys_2::LLAMA_FLASH_ATTN_TYPE_ENABLED;
use log::error;
use log::info;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ChatTemplateDoesNotCompileParams;
use paddler_types::agent_issue_params::SlotCannotStartParams;
use paddler_types::chat_template::ChatTemplate;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::model_metadata::ModelMetadata;
use tokio::sync::oneshot;

use crate::agent::llamacpp_arbiter_handle::LlamaCppArbiterHandle;
use crate::agent::llamacpp_slot::LlamaCppSlot;
use crate::agent::llamacpp_slot_context::LlamaCppSlotContext;
use crate::agent::model_metadata_holder::ModelMetadataHolder;
use crate::agent_issue_fix::AgentIssueFix;
use crate::chat_template_renderer::ChatTemplateRenderer;
use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType;
use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

pub struct LlamaCppArbiter {
    pub agent_name: Option<String>,
    pub chat_template_override: Option<ChatTemplate>,
    pub desired_slots_total: i32,
    pub inference_parameters: InferenceParameters,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub model_path: PathBuf,
    pub model_path_string: String,
    pub slot_aggregated_status_manager: Arc<SlotAggregatedStatusManager>,
}

impl LlamaCppArbiter {
    pub async fn spawn(&self) -> Result<LlamaCppArbiterHandle> {
        let (chat_template_loaded_tx, chat_template_loaded_rx) = oneshot::channel::<()>();
        let (llamacpp_slot_addr_tx, llamacpp_slot_addr_rx) = oneshot::channel();
        let (model_loaded_tx, model_loaded_rx) = oneshot::channel::<()>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let available_parallelism: i32 = available_parallelism()?.get().try_into()?;
        let n_threads = max(2, available_parallelism / 2);
        let n_threads_batch = max(2, available_parallelism / 2);

        info!("Using threads for parallelism threads/batch: {n_threads}/{n_threads_batch}");

        let agent_name_clone = self.agent_name.clone();
        let desired_slots_total = self.desired_slots_total;
        let inference_parameters = self.inference_parameters.clone();
        let model_metadata_holder = self.model_metadata_holder.clone();
        let model_path = self.model_path.clone();
        let model_path_string_clone = self.model_path_string.clone();
        let model_path_string = self.model_path_string.clone();
        let chat_template_override = self.chat_template_override.clone();
        let slot_aggregated_status_manager = self.slot_aggregated_status_manager.clone();

        let sync_arbiter_thread_handle = thread::spawn(move || -> Result<()> {
            let llama_backend =
                Arc::new(LlamaBackend::init().context("Unable to initialize llama.cpp backend")?);
            let mut context_params = LlamaContextParams::default()
                .with_embeddings(inference_parameters.enable_embeddings)
                .with_n_ctx(NonZeroU32::new(inference_parameters.context_size))
                .with_flash_attention_policy(LLAMA_FLASH_ATTN_TYPE_ENABLED)
                .with_n_threads(n_threads)
                .with_n_threads_batch(n_threads_batch)
                .with_pooling_type(
                    inference_parameters
                        .pooling_type
                        .clone()
                        .to_llama_pooling_type(),
                );

            if inference_parameters.enable_embeddings {
                context_params =
                    context_params.with_n_seq_max(inference_parameters.embedding_n_seq_max);
            }

            let llama_ctx_params = Arc::new(context_params);
            let backend_clone = llama_backend.clone();
            let model = Arc::new(
                LlamaModel::load_from_file(&backend_clone.clone(), model_path.clone(), &{
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

            for i in 0..model.meta_count() {
                model_metadata
                    .set_meta_field(model.meta_key_by_index(i)?, model.meta_val_str_by_index(i)?);
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
                            .register_fix(AgentIssueFix::ChatTemplateIsCompiled);

                        renderer
                    }
                    Err(err) => {
                        slot_aggregated_status_manager
                            .slot_aggregated_status
                            .register_issue(AgentIssue::ChatTemplateDoesNotCompile(
                                ChatTemplateDoesNotCompileParams {
                                    error: format!("{err}"),
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

            let slot_index = Arc::new(AtomicU32::new(0));
            let mut special_token_decoder = encoding_rs::UTF_8.new_decoder();
            let slot_context = Arc::new(LlamaCppSlotContext {
                agent_name: agent_name_clone,
                chat_template_renderer,
                inference_parameters,
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
                model,
                model_path,
            });
            let system = System::new();

            system.block_on(async move {
                llamacpp_slot_addr_tx
                    .send(SyncArbiter::start(
                        desired_slots_total as usize,
                        move || {
                            let index = slot_index.fetch_add(1, Ordering::SeqCst);
                            let llamacpp_slot = LlamaCppSlot::new(
                                index,
                                llama_backend.clone(),
                                llama_ctx_params.clone(),
                                slot_context.clone(),
                                slot_aggregated_status_manager.bind_slot_status(),
                            );

                            match llamacpp_slot {
                                Err(err) => {
                                    slot_aggregated_status_manager
                                        .slot_aggregated_status
                                        .register_issue(AgentIssue::SlotCannotStart(
                                            SlotCannotStartParams {
                                                error: format!("{err}"),
                                                slot_index: index,
                                            },
                                        ));

                                    panic!("Failed to create slot: {err}");
                                }
                                Ok(llamacpp_slot) => {
                                    slot_aggregated_status_manager
                                        .slot_aggregated_status
                                        .register_fix(AgentIssueFix::SlotStarted(index));

                                    llamacpp_slot
                                }
                            }
                        },
                    ))
                    .expect("Failed to send LlamaCppSlot address");

                shutdown_rx
                    .await
                    .expect("Failed to receive shutdown signal");

                System::current().stop();
            });

            Ok(())
        });

        match model_loaded_rx
            .await
            .context("Failed to receive model loaded signal")
        {
            Ok(()) => {
                self.slot_aggregated_status_manager
                    .slot_aggregated_status
                    .register_fix(AgentIssueFix::ModelIsLoaded);
            }
            Err(err) => {
                error!("Failed to load model: {err}");

                self.slot_aggregated_status_manager
                    .slot_aggregated_status
                    .register_issue(AgentIssue::ModelCannotBeLoaded(model_path_string.clone()));
            }
        }

        match chat_template_loaded_rx
            .await
            .context("Failed to receive chat template loaded signal")
        {
            Ok(()) => {
                self.slot_aggregated_status_manager
                    .slot_aggregated_status
                    .register_fix(AgentIssueFix::ModelChatTemplateIsLoaded);
            }
            Err(err) => {
                error!("Failed to load chat template: {err}");

                if !self
                    .slot_aggregated_status_manager
                    .slot_aggregated_status
                    .has_issue(&AgentIssue::ModelCannotBeLoaded(model_path_string.clone()))
                {
                    // If the model cannot be loaded, that doesn't mean that the chat template
                    // cannot be loaded.
                    self.slot_aggregated_status_manager
                        .slot_aggregated_status
                        .register_issue(AgentIssue::UnableToFindChatTemplate(
                            model_path_string.clone(),
                        ));
                }
            }
        }

        Ok(LlamaCppArbiterHandle {
            llamacpp_slot_addr: llamacpp_slot_addr_rx
                .await
                .context("Unable to await for llamacpp slot addr")?,
            shutdown_tx,
            sync_arbiter_thread_handle,
        })
    }
}

#[cfg(test)]
#[cfg(feature = "tests_that_use_llms")]
mod tests {
    use futures::future::join_all;
    use paddler_types::agent_desired_model::AgentDesiredModel;
    use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_types::inference_parameters::InferenceParameters;
    use paddler_types::request_params::ContinueFromRawPromptParams;
    use tokio::sync::mpsc;

    use super::*;
    use crate::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
    use crate::agent_desired_state::AgentDesiredState;
    use crate::converts_to_applicable_state::ConvertsToApplicableState as _;

    const SLOTS_TOTAL: i32 = 2;

    #[actix_web::test]
    async fn test_llamacpp_arbiter_spawn() -> Result<()> {
        let desired_state = AgentDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
                filename: "Qwen3-0.6B-Q8_0.gguf".to_string(),
                repo_id: "Qwen/Qwen3-0.6B-GGUF".to_string(),
                revision: "main".to_string(),
            }),
        };
        let slot_aggregated_status_manager =
            Arc::new(SlotAggregatedStatusManager::new(SLOTS_TOTAL));

        let applicable_state = desired_state
            .to_applicable_state(
                slot_aggregated_status_manager
                    .slot_aggregated_status
                    .clone(),
            )
            .await?
            .expect("Failed to convert to applicable state");

        let model_path = applicable_state.model_path.expect("Model path is required");
        let llamacpp_arbiter = LlamaCppArbiter {
            agent_name: Some("test_agent".to_string()),
            chat_template_override: None,
            desired_slots_total: SLOTS_TOTAL,
            inference_parameters: applicable_state.inference_parameters,
            model_metadata_holder: Arc::new(ModelMetadataHolder::new()),
            model_path: model_path.clone(),
            model_path_string: model_path.display().to_string(),
            slot_aggregated_status_manager,
        };
        let controller = llamacpp_arbiter.spawn().await?;

        let raw_prompt =
            "<|im_start|>user\nHow can I make a cat happy?<|im_end|>\n<|im_start|>assistant\n";
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let (_, generate_tokens_stop_rx_1) = mpsc::unbounded_channel::<()>();
        let (_, generate_tokens_stop_rx_2) = mpsc::unbounded_channel::<()>();
        let (_, generate_tokens_stop_rx_3) = mpsc::unbounded_channel::<()>();

        let futures = vec![
            controller
                .llamacpp_slot_addr
                .send(ContinueFromRawPromptRequest {
                    generated_tokens_tx: generated_tokens_tx.clone(),
                    generate_tokens_stop_rx: generate_tokens_stop_rx_1,
                    params: ContinueFromRawPromptParams {
                        max_tokens: 30,
                        raw_prompt: raw_prompt.to_string(),
                    },
                }),
            controller
                .llamacpp_slot_addr
                .send(ContinueFromRawPromptRequest {
                    generated_tokens_tx: generated_tokens_tx.clone(),
                    generate_tokens_stop_rx: generate_tokens_stop_rx_2,
                    params: ContinueFromRawPromptParams {
                        max_tokens: 30,
                        raw_prompt: raw_prompt.to_string(),
                    },
                }),
            controller
                .llamacpp_slot_addr
                .send(ContinueFromRawPromptRequest {
                    generated_tokens_tx,
                    generate_tokens_stop_rx: generate_tokens_stop_rx_3,
                    params: ContinueFromRawPromptParams {
                        max_tokens: 30,
                        raw_prompt: raw_prompt.to_string(),
                    },
                }),
        ];

        tokio::spawn(async move {
            while let Some(generated_token) = generated_tokens_rx.recv().await {
                println!("Received generated token: {generated_token:?}");
            }
        });

        let results = join_all(futures).await;

        for result in results {
            if let Err(err) = result {
                eprintln!("Error generating response: {err}");
            }
        }

        controller.shutdown()?;

        Ok(())
    }
}
