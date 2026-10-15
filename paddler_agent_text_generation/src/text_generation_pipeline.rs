use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;

use encoding_rs::Decoder;
use encoding_rs::UTF_8;
use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::TokenToStringError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdContextParams;
use llama_cpp_bindings::mtmd::micro_batch_tokens;
use llama_cpp_bindings::token::LlamaToken;
use log::info;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_runtime::continue_startup_unless_shutting_down::continue_startup_unless_shutting_down;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_thread_count::inference_thread_count;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;
use paddler_agent_runtime::scheduler_ready::SchedulerReady;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_runtime::start_scheduler_thread::start_scheduler_thread;
use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::media_marker::MediaMarker;

use crate::chat_prompt_renderer::ChatPromptRenderer;
use crate::chat_template_renderer::ChatTemplateRenderer;
use crate::continuous_batch_scheduler::ContinuousBatchScheduler;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use crate::generation_request_preparer::GenerationRequestPreparer;
use crate::image_input::ImageInput;
use crate::json_tools_serializer::JsonToolsSerializer;
use crate::llama_cpp_media_marker::LlamaCppMediaMarker;
use crate::model_bos_token::ModelBosToken;
use crate::multimodal_projection::MultimodalProjection;
use crate::multimodal_prompt_support::MultimodalPromptSupport;
use crate::prompt_tokenizer::PromptTokenizer;
use crate::provides_media_marker::ProvidesMediaMarker;
use crate::sampler_chain_factory::SamplerChainFactory;
use crate::text_generation_error::TextGenerationError;
use crate::text_generation_settings::TextGenerationSettings;
use crate::token_generation_support::TokenGenerationSupport;

fn special_token_text(
    model: &LlamaModel,
    special_token: Option<LlamaToken>,
    decoder: &mut Decoder,
) -> Result<String, TokenToStringError> {
    special_token.map_or_else(
        || Ok(String::new()),
        |token| model.token_to_piece(&SampledToken::Content(token), decoder, true, None),
    )
}

pub struct TextGenerationPipeline<TMediaMarker: ProvidesMediaMarker = LlamaCppMediaMarker> {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub media_marker: TMediaMarker,
    pub model_path: PathBuf,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub text_generation_settings: TextGenerationSettings,
}

impl<TMediaMarker: ProvidesMediaMarker> TextGenerationPipeline<TMediaMarker> {
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<SchedulerSpawnOutcome<GenerationRequestPreparer>, TextGenerationError> {
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

        continue_startup_unless_shutting_down(agent_shutdown, || {
            let thread_count = inference_thread_count(
                self.inference_runtime_context
                    .available_parallelism
                    .as_ref(),
            )?;
            let loaded_llama_model = LoadedLlamaModel::load(
                &self.inference_runtime_context,
                &self.model_path,
                &LlamaModelParams::default()
                    .with_n_gpu_layers(self.model_runtime_parameters.n_gpu_layers),
            )?;

            continue_startup_unless_shutting_down(agent_shutdown, || {
                let token_generation_support =
                    self.token_generation_support(&loaded_llama_model.model)?;

                let mut llama_context = loaded_llama_model.create_llama_context(
                    &self.inference_runtime_context,
                    LlamaContextSettings {
                        model_runtime_parameters: &self.model_runtime_parameters,
                        n_seq_max: u32::from(desired_slots_total),
                        thread_count,
                    }
                    .into_llama_context_params(),
                )?;
                let sequence_context_size = llama_context.n_ctx_seq();
                let prompt_tokenizer = PromptTokenizer {
                    model: loaded_llama_model.model.clone(),
                    model_bos_token: ModelBosToken::of(&loaded_llama_model.model),
                    sequence_context_size,
                };

                continue_startup_unless_shutting_down(agent_shutdown, || {
                    let image_input = self.image_input(
                        &loaded_llama_model.model,
                        &llama_context,
                        &prompt_tokenizer,
                        &token_generation_support.chat_prompt_renderer,
                    )?;
                    let mut batch = LlamaBatch::new(
                        self.model_runtime_parameters.n_batch.capacity(),
                        i32::from(desired_slots_total),
                    );

                    continue_startup_unless_shutting_down(agent_shutdown, || {
                        loaded_llama_model.warm_up_llama_context(
                            &mut llama_context,
                            &mut batch,
                            desired_slots_total,
                        );

                        let (scheduler_message_tx, scheduler_message_rx) = channel();

                        run_scheduler_unless_startup_abandoned(
                            scheduler_startup_tx,
                            Ok(SchedulerReady {
                                model_metadata: loaded_llama_model.model_metadata.clone(),
                                request_preparer: Arc::new(SchedulerRequestPreparer {
                                    agent_name: self.inference_runtime_context.agent_name.clone(),
                                    preparation: GenerationRequestPreparer {
                                        image_input,
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
                                        token_generation_support,
                                        tools_serializer: JsonToolsSerializer,
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
                                        model: loaded_llama_model.model.clone(),
                                        n_batch: self.model_runtime_parameters.n_batch,
                                        sequence_context_size,
                                    },
                                })
                                .run();
                            },
                        );

                        Ok(())
                    })
                })
            })
        })
    }

    fn token_generation_support(
        &self,
        model: &LlamaModel,
    ) -> Result<TokenGenerationSupport, TextGenerationError> {
        let chat_template = self.chat_template(model)?;
        let chat_template_renderer = self.chat_template_renderer(chat_template)?;
        let mut special_token_decoder = UTF_8.new_decoder();

        Ok(TokenGenerationSupport {
            chat_prompt_renderer: ChatPromptRenderer {
                chat_template_renderer,
                media_marker: MediaMarker::new(
                    self.media_marker
                        .media_marker()
                        .map_err(TextGenerationError::MediaMarkerUnavailable)?
                        .to_owned(),
                ),
                token_bos_str: special_token_text(
                    model,
                    model.token_bos(),
                    &mut special_token_decoder,
                )
                .map_err(TextGenerationError::SpecialTokenUnrenderable)?,
                token_eos_str: special_token_text(
                    model,
                    model.token_eos(),
                    &mut special_token_decoder,
                )
                .map_err(TextGenerationError::SpecialTokenUnrenderable)?,
                token_nl_str: special_token_text(
                    model,
                    model.token_nl(),
                    &mut special_token_decoder,
                )
                .map_err(TextGenerationError::SpecialTokenUnrenderable)?,
            },
            streaming_markers: model
                .streaming_markers()
                .map_err(TextGenerationError::StreamingMarkersUnavailable)?,
        })
    }

    fn chat_template(&self, model: &LlamaModel) -> Result<ChatTemplate, TextGenerationError> {
        let model_issue_path = ModelPath::from(self.model_path.as_path());
        let chat_template = match &self.text_generation_settings.chat_template_source {
            ChatTemplateSource::Override(chat_template_override) => {
                Ok(chat_template_override.clone())
            }
            ChatTemplateSource::EmbeddedInModel => model
                .chat_template(None)
                .map_err(TextGenerationError::ChatTemplateUnavailable)
                .and_then(|model_chat_template| {
                    model_chat_template
                        .to_string()
                        .map_err(TextGenerationError::ChatTemplateNotUtf8)
                })
                .map(|content| ChatTemplate { content }),
        };
        let slot_aggregated_status = &self.inference_runtime_context.slot_aggregated_status;

        match &chat_template {
            Ok(_) => {
                slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelChatTemplateIsLoaded(model_issue_path));
            }
            Err(_chat_template_unavailable) => {
                slot_aggregated_status.register_pipeline_issue(
                    AgentIssue::UnableToFindChatTemplate(model_issue_path),
                );
            }
        }

        chat_template
    }

    fn chat_template_renderer(
        &self,
        chat_template: ChatTemplate,
    ) -> Result<ChatTemplateRenderer, TextGenerationError> {
        let model_issue_path = ModelPath::from(self.model_path.as_path());
        let template_content = chat_template.content.clone();
        let slot_aggregated_status = &self.inference_runtime_context.slot_aggregated_status;

        match ChatTemplateRenderer::new(chat_template) {
            Ok(chat_template_renderer) => {
                slot_aggregated_status
                    .register_fix(&AgentIssueFix::ChatTemplateIsCompiled(model_issue_path));

                Ok(chat_template_renderer)
            }
            Err(compile_error) => {
                slot_aggregated_status.register_pipeline_issue(
                    AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
                        error: format!("{compile_error}"),
                        model_path: model_issue_path,
                        template_content,
                    }),
                );

                Err(TextGenerationError::ChatTemplateDoesNotCompile(
                    compile_error,
                ))
            }
        }
    }

    fn image_input(
        &self,
        model: &LlamaModel,
        llama_context: &LlamaContext<'_>,
        prompt_tokenizer: &PromptTokenizer,
        chat_prompt_renderer: &ChatPromptRenderer,
    ) -> Result<ImageInput, TextGenerationError> {
        let MultimodalProjection::File(multimodal_projection_path) =
            &self.text_generation_settings.multimodal_projection
        else {
            return Ok(ImageInput::Unsupported);
        };
        let multimodal_projection_issue_path =
            ModelPath::from(multimodal_projection_path.as_path());
        let slot_aggregated_status = &self.inference_runtime_context.slot_aggregated_status;

        match MtmdContext::init_from_file(
            &multimodal_projection_path.to_string_lossy(),
            model,
            &MtmdContextParams::default(),
        ) {
            Ok(multimodal_context) => {
                slot_aggregated_status.register_fix(&AgentIssueFix::MultimodalProjectionIsLoaded(
                    multimodal_projection_issue_path,
                ));

                info!(
                    "Multimodal context initialized from: {}",
                    multimodal_projection_path.display()
                );

                Ok(ImageInput::Supported(MultimodalPromptSupport {
                    add_special_tokens: !prompt_tokenizer
                        .template_renders_its_own_bos_token(chat_prompt_renderer)?,
                    micro_batch_tokens: micro_batch_tokens(
                        llama_context,
                        self.model_runtime_parameters.n_batch.tokens(),
                    ),
                    multimodal_context: Arc::new(multimodal_context),
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
