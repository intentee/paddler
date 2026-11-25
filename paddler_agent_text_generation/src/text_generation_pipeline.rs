use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_runtime::continue_startup_unless_shutting_down::continue_startup_unless_shutting_down;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_ready::SchedulerReady;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::start_scheduler_thread::start_scheduler_thread;
use paddler_agent_runtime::startup_outcome::StartupOutcome;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_messaging::inference_capability::InferenceCapability;

use crate::continuous_batch_scheduler::ContinuousBatchScheduler;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use crate::generation_request_preparer::GenerationRequestPreparer;
use crate::model_bos_token::ModelBosToken;
use crate::multimodal_ingestion::MultimodalIngestion;
use crate::prompt_tokenizer::PromptTokenizer;
use crate::sampler_chain_factory::SamplerChainFactory;
use crate::text_generation_error::TextGenerationError;
use crate::text_generation_settings::TextGenerationSettings;

pub struct TextGenerationPipeline {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
    pub model_runtime_parameters: DesiredModelRuntimeParameters,
    pub text_generation_settings: TextGenerationSettings,
}

impl TextGenerationPipeline {
    /// # Errors
    ///
    /// Returns [`TextGenerationError::AgentRuntime`] when the text generation context cannot claim the VRAM, be created or be warmed up, and [`TextGenerationError::SamplingParametersUnfitForTheSequenceContext`] when the penalty window exceeds the sequence context.
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
            let multimodal_ingestion = MultimodalIngestion::for_context(
                &llama_context,
                Arc::clone(&self.model_capabilities),
            );
            let sequence_context_size = multimodal_ingestion.sequence_context_size;
            let sampling_parameters = self
                .text_generation_settings
                .sampling_parameters
                .resolve(&self.model_capabilities.sampling_defaults);

            sampling_parameters
                .ensure_penalty_window_fits(sequence_context_size)
                .map_err(TextGenerationError::SamplingParametersUnfitForTheSequenceContext)?;

            let prompt_tokenizer = PromptTokenizer {
                model: loaded_llama_model.model.clone(),
                model_bos_token: ModelBosToken::of(&loaded_llama_model.model),
                sequence_context_size,
            };

            continue_startup_unless_shutting_down(agent_shutdown, || {
                loaded_llama_model
                    .warm_up_and_serve(llama_context, |llama_context, batch| {
                        let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

                        run_scheduler_unless_startup_abandoned(
                            scheduler_startup_tx,
                            Ok(SchedulerReady {
                                request_preparer: Arc::new(SchedulerRequestPreparer {
                                    agent_name: self.inference_runtime_context.agent_name.clone(),
                                    preparation: GenerationRequestPreparer {
                                        image_resize_to_fit: self
                                            .text_generation_settings
                                            .image_resize_to_fit,
                                        loaded_llama_model: loaded_llama_model.clone(),
                                        model_capabilities: self.model_capabilities.clone(),
                                        prompt_tokenizer,
                                        sampler_chain_factory: SamplerChainFactory {
                                            sampling_parameters,
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
                                        multimodal_ingestion,
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
    }
}
