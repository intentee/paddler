use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;

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
use llama_cpp_bindings::mtmd::mtmd_default_marker;
use llama_cpp_bindings::token::LlamaToken;
use log::info;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::await_scheduler_startup::await_scheduler_startup;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_thread_count::inference_thread_count;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_runtime::send_startup_signal::send_startup_signal;
use paddler_agent_runtime::startup_signal_delivery::StartupSignalDelivery;
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
use crate::model_bos_token::ModelBosToken;
use crate::multimodal_projection::MultimodalProjection;
use crate::multimodal_prompt_support::MultimodalPromptSupport;
use crate::prompt_tokenizer::PromptTokenizer;
use crate::sampler_chain_factory::SamplerChainFactory;
use crate::text_generation_error::TextGenerationError;
use crate::text_generation_settings::TextGenerationSettings;
use crate::token_generation_support::TokenGenerationSupport;

fn special_token_text(
    model: &LlamaModel,
    token: LlamaToken,
    decoder: &mut Decoder,
) -> Result<String, TokenToStringError> {
    model.token_to_piece(&SampledToken::Content(token), decoder, true, None)
}

pub struct TextGenerationPipeline {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub model_path: PathBuf,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub text_generation_settings: TextGenerationSettings,
}

impl TextGenerationPipeline {
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<
        SchedulerSpawnOutcome<GenerationRequestPreparer, TextGenerationError>,
        TextGenerationError,
    > {
        let (scheduler_ready_tx, scheduler_ready_rx) = oneshot::channel();
        let agent_shutdown = cancellation_token.clone();

        let scheduler_thread_handle =
            thread::spawn(move || self.run_scheduler(agent_shutdown, scheduler_ready_tx));

        await_scheduler_startup(
            cancellation_token,
            scheduler_ready_rx,
            scheduler_thread_handle,
        )
        .await
    }

    fn run_scheduler(
        self,
        agent_shutdown: CancellationToken,
        scheduler_ready_tx: oneshot::Sender<
            Arc<SchedulerRequestPreparer<GenerationRequestPreparer>>,
        >,
    ) -> Result<(), TextGenerationError> {
        let desired_slots_total = self
            .inference_runtime_context
            .slot_aggregated_status
            .desired_slots_total;
        let thread_count = inference_thread_count()?;
        let loaded_llama_model = LoadedLlamaModel::load(
            &self.inference_runtime_context,
            &self.model_path,
            &LlamaModelParams::default()
                .with_n_gpu_layers(self.model_runtime_parameters.n_gpu_layers),
        )?;
        let token_generation_support = self.token_generation_support(&loaded_llama_model.model)?;

        self.inference_runtime_context
            .slot_aggregated_status
            .set_model_path(Some(ModelPath::from(self.model_path.as_path()).model_path));

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
        let image_input = self.image_input(
            &loaded_llama_model.model,
            &llama_context,
            sequence_context_size,
        )?;
        let mut batch = LlamaBatch::new(
            self.model_runtime_parameters.n_batch.tokens_usize(),
            i32::from(desired_slots_total),
        )
        .map_err(AgentRuntimeError::BatchAllocationFailed)?;

        loaded_llama_model.warm_up_llama_context(
            &mut llama_context,
            &mut batch,
            desired_slots_total,
        );

        let (scheduler_message_tx, scheduler_message_rx) = channel();
        let request_preparer = Arc::new(SchedulerRequestPreparer {
            agent_name: self.inference_runtime_context.agent_name.clone(),
            preparation: GenerationRequestPreparer {
                image_input,
                image_resize_to_fit: self.text_generation_settings.image_resize_to_fit,
                loaded_llama_model: loaded_llama_model.clone(),
                prompt_tokenizer: PromptTokenizer {
                    model: loaded_llama_model.model.clone(),
                    model_bos_token: ModelBosToken::of(&loaded_llama_model.model),
                    sequence_context_size,
                },
                sampler_chain_factory: SamplerChainFactory {
                    sampling_parameters: self.text_generation_settings.sampling_parameters.clone(),
                    n_vocab: loaded_llama_model.model.n_vocab(),
                },
                token_generation_support,
            },
            preparation_tasks: TaskTracker::new(),
            scheduler_message_tx,
        });

        if matches!(
            send_startup_signal(scheduler_ready_tx, request_preparer),
            StartupSignalDelivery::Abandoned
        ) {
            return Ok(());
        }

        ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
            agent_shutdown,
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

        Ok(())
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
                    mtmd_default_marker()
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
                slot_aggregated_status
                    .register_issue(AgentIssue::UnableToFindChatTemplate(model_issue_path));
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
                slot_aggregated_status.register_issue(AgentIssue::ChatTemplateDoesNotCompile(
                    ChatTemplateDoesNotCompileParams {
                        error: format!("{compile_error}"),
                        model_path: model_issue_path,
                        template_content,
                    },
                ));

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
        sequence_context_size: u32,
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
                    micro_batch_tokens: micro_batch_tokens(
                        llama_context,
                        self.model_runtime_parameters.n_batch.tokens(),
                    ),
                    multimodal_context: Arc::new(multimodal_context),
                    n_batch: self.model_runtime_parameters.n_batch.tokens_i32(),
                    sequence_context_size,
                }))
            }
            Err(source) => {
                slot_aggregated_status.register_issue(
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
