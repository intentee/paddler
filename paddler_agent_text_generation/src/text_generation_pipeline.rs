use std::sync::Arc;

use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::model::llama_chat_template::LlamaChatTemplate;
use log::info;
use log::warn;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_runtime::continue_startup_unless_shutting_down::continue_startup_unless_shutting_down;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_ready::SchedulerReady;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::start_scheduler_thread::start_scheduler_thread;
use paddler_agent_runtime::startup_outcome::StartupOutcome;
use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::inference_capability::InferenceCapability;

use crate::continuous_batch_scheduler::ContinuousBatchScheduler;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use crate::conversation_prompt_support::ConversationPromptSupport;
use crate::conversation_support::ConversationSupport;
use crate::generation_request_preparer::GenerationRequestPreparer;
use crate::image_input::ImageInput;
use crate::model_bos_token::ModelBosToken;
use crate::multimodal_projection::MultimodalProjection;
use crate::multimodal_prompt_support::MultimodalPromptSupport;
use crate::prompt_tokenizer::PromptTokenizer;
use crate::resident_multimodal_context::ResidentMultimodalContext;
use crate::sampler_chain_factory::SamplerChainFactory;
use crate::text_generation_error::TextGenerationError;
use crate::text_generation_settings::TextGenerationSettings;

pub struct TextGenerationPipeline {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub text_generation_settings: TextGenerationSettings,
}

impl TextGenerationPipeline {
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<StartupOutcome<SchedulerHandle<GenerationRequestPreparer>>, TextGenerationError>
    {
        start_scheduler_thread(
            cancellation_token,
            move |agent_shutdown, scheduler_startup_tx| {
                self.run_scheduler(agent_shutdown, scheduler_startup_tx)
            },
        )
        .await
    }

    fn run_scheduler(
        self,
        agent_shutdown: &CancellationToken,
        scheduler_startup_tx: &UnboundedSender<
            Result<SchedulerReady<GenerationRequestPreparer>, TextGenerationError>,
        >,
    ) -> Result<(), TextGenerationError> {
        let desired_slots_total = self
            .inference_runtime_context
            .slot_aggregated_status
            .desired_slots_total;
        let loaded_llama_model = self.loaded_llama_model.clone();

        continue_startup_unless_shutting_down(agent_shutdown, || {
            let llama_context = loaded_llama_model.create_llama_context(
                &self.inference_runtime_context,
                InferenceCapability::TextGeneration,
                loaded_llama_model
                    .llama_context_settings(
                        &self.model_runtime_parameters,
                        u32::from(desired_slots_total),
                    )
                    .into_llama_context_params(),
            )?;
            let sequence_context_size = llama_context.n_ctx_seq();

            self.text_generation_settings
                .sampling_parameters
                .ensure_penalty_window_fits(sequence_context_size)
                .map_err(TextGenerationError::SamplingParametersUnfitForTheSequenceContext)?;

            let prompt_tokenizer = PromptTokenizer {
                model: loaded_llama_model.model.clone(),
                model_bos_token: ModelBosToken::of(&loaded_llama_model.model),
                sequence_context_size,
            };

            continue_startup_unless_shutting_down(agent_shutdown, || {
                let conversation_support =
                    self.conversation_support(&llama_context, &prompt_tokenizer)?;

                continue_startup_unless_shutting_down(agent_shutdown, || {
                    loaded_llama_model
                        .warm_up_and_serve(llama_context, |llama_context, batch| {
                            let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

                            run_scheduler_unless_startup_abandoned(
                                scheduler_startup_tx,
                                Ok(SchedulerReady {
                                    request_preparer: Arc::new(SchedulerRequestPreparer {
                                        agent_name: self
                                            .inference_runtime_context
                                            .agent_name
                                            .clone(),
                                        preparation: GenerationRequestPreparer {
                                            conversation_support,
                                            image_resize_to_fit: self
                                                .text_generation_settings
                                                .image_resize_to_fit,
                                            loaded_llama_model: loaded_llama_model.clone(),
                                            prompt_tokenizer,
                                            sampler_chain_factory: SamplerChainFactory {
                                                sampling_parameters: self
                                                    .text_generation_settings
                                                    .sampling_parameters
                                                    .clone(),
                                                n_vocab: loaded_llama_model.model.n_vocab(),
                                            },
                                        },
                                        preparation_tasks: TaskTracker::new(),
                                        scheduler_message_tx,
                                    }),
                                }),
                                || {
                                    ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
                                        agent_shutdown: agent_shutdown.clone(),
                                        batch,
                                        scheduler_message_rx,
                                        llama_context,
                                        scheduler_context: ContinuousBatchSchedulerContext {
                                            agent_name: self.inference_runtime_context.agent_name,
                                            desired_slots_total,
                                            model: &loaded_llama_model.model,
                                            sequence_context_size,
                                        },
                                    })
                                    .run();
                                },
                            );
                        })
                        .map_err(TextGenerationError::from)
                })
            })
        })
    }

    fn conversation_support(
        &self,
        llama_context: &LlamaContext<'_>,
        prompt_tokenizer: &PromptTokenizer,
    ) -> Result<ConversationSupport, TextGenerationError> {
        let chat_template = match &self.text_generation_settings.chat_template_source {
            ChatTemplateSource::Override(chat_template_override) => chat_template_override.clone(),
            ChatTemplateSource::EmbeddedInModel => {
                let Some(embedded_chat_template) = self
                    .loaded_llama_model
                    .model_metadata
                    .embedded_chat_template()
                else {
                    warn!(
                        "{:?}: the model embeds no chat template, so it serves no conversations",
                        self.inference_runtime_context.agent_name
                    );

                    return Ok(ConversationSupport::ChatTemplateMissing(
                        self.loaded_llama_model
                            .llama_model_settings
                            .reported_model_path(),
                    ));
                };

                embedded_chat_template
            }
        };
        let chat_templates = self.chat_templates(chat_template)?;

        Ok(ConversationSupport::Available(Box::new(
            ConversationPromptSupport {
                chat_templates,
                image_input: self.image_input(llama_context, prompt_tokenizer)?,
            },
        )))
    }

    fn prepared_chat_templates(
        &self,
        chat_template: &ChatTemplate,
    ) -> Result<ChatTemplates, TextGenerationError> {
        let model = &self.loaded_llama_model.model;

        match &self.text_generation_settings.chat_template_source {
            ChatTemplateSource::Override(_) => ChatTemplates::with_override(
                model,
                &LlamaChatTemplate::new(&chat_template.content)
                    .map_err(TextGenerationError::ChatTemplateContainsNul)?,
            ),
            ChatTemplateSource::EmbeddedInModel => ChatTemplates::new(model),
        }
        .map_err(TextGenerationError::ChatTemplateDoesNotCompile)
    }

    fn chat_templates(
        &self,
        chat_template: ChatTemplate,
    ) -> Result<ChatTemplates, TextGenerationError> {
        let model_issue_path = self
            .loaded_llama_model
            .llama_model_settings
            .reported_model_path();
        let slot_aggregated_status = &self.inference_runtime_context.slot_aggregated_status;

        match self.prepared_chat_templates(&chat_template) {
            Ok(chat_templates) => {
                slot_aggregated_status
                    .register_fix(&AgentIssueFix::ChatTemplateIsCompiled(model_issue_path));

                Ok(chat_templates)
            }
            Err(preparation_error) => {
                slot_aggregated_status.register_pipeline_issue(
                    AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
                        error: preparation_error.to_string(),
                        model_path: model_issue_path,
                        template_content: chat_template.content,
                    }),
                );

                Err(preparation_error)
            }
        }
    }

    fn image_input(
        &self,
        llama_context: &LlamaContext<'_>,
        prompt_tokenizer: &PromptTokenizer,
    ) -> Result<ImageInput, TextGenerationError> {
        let MultimodalProjection::File(multimodal_projection_path) =
            &self.text_generation_settings.multimodal_projection
        else {
            return Ok(ImageInput::Unsupported);
        };
        let multimodal_projection_issue_path =
            ModelPath::from(multimodal_projection_path.as_path());
        let slot_aggregated_status = &self.inference_runtime_context.slot_aggregated_status;
        let vram_occupancy = self
            .inference_runtime_context
            .vram_residencies
            .multimodal_projection
            .try_occupy()?;

        match ResidentMultimodalContext::load(
            multimodal_projection_path,
            &self.loaded_llama_model.model,
            vram_occupancy,
        ) {
            Ok(resident_multimodal_context) => {
                slot_aggregated_status.register_fix(&AgentIssueFix::MultimodalProjectionIsLoaded(
                    multimodal_projection_issue_path,
                ));

                info!(
                    "Multimodal context initialized from: {}",
                    multimodal_projection_path.display()
                );

                Ok(ImageInput::Supported(MultimodalPromptSupport {
                    micro_batch_tokens: ContextLimits::of(llama_context)
                        .micro_batch_tokens(self.model_runtime_parameters.n_batch.tokens()),
                    multimodal_context: Arc::new(resident_multimodal_context),
                    n_batch: self.model_runtime_parameters.n_batch.tokens_i32(),
                    sequence_context_size: prompt_tokenizer.sequence_context_size,
                }))
            }
            Err(source) => {
                slot_aggregated_status.register_pipeline_issue(
                    AgentIssue::MultimodalProjectionCannotBeLoaded(
                        multimodal_projection_issue_path,
                    ),
                );

                Err(TextGenerationError::MultimodalProjectionLoadFailed {
                    multimodal_projection_path: multimodal_projection_path.clone(),
                    source,
                })
            }
        }
    }
}
