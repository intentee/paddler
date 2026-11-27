use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Error;
use anyhow::Result;
use llama_cpp_context::llama_batch::LlamaBatch;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::conversation_generation::ConversationGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::multimodal_ingestion::MultimodalIngestion;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_file_fixture::image_fixture::ImageFixture;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;
use paddler_model_card_fixture::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

use crate::default_model_capabilities_settings::default_model_capabilities_settings;
use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use crate::single_image_question::SingleImageQuestion;

const IMAGE_QUESTION: &str = "How many llamas are in the image?";

fn undelivered_to_the_scheduler(
    send_error: &SendError<SchedulerMessage<PreparedGenerationRequest>>,
) -> Error {
    Error::msg(send_error.to_string())
}

pub enum ImageQuestionScheduling {
    WithAMultimodalProjection,
    WithoutAMultimodalProjection,
}

impl ImageQuestionScheduling {
    /// # Errors
    ///
    /// Returns an error when `SmolVLM2` or its multimodal projection is not cached or cannot load, when the capabilities the image question is prepared and scheduled with cannot be established, when the image question cannot be prepared or handed to the scheduler, and when the scheduler does not answer it.
    pub fn first_generated_token_result(self) -> Result<GeneratedTokenResult> {
        let runtime = smolvlm2_256m()
            .cached_path()
            .context("SmolVLM2 must be in the Hugging Face cache")
            .and_then(|model_path| {
                LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
                    .context("SmolVLM2 must load")
            })?;
        let llama_context = runtime
            .create_text_generation_llama_context(1)
            .context("the text generation context must be created")?;
        let sequence_context_size = llama_context.n_ctx_seq();
        let image_question_preparer = smolvlm2_256m_mmproj()
            .cached_path()
            .context("the multimodal projection must be in the Hugging Face cache")
            .and_then(|multimodal_projection_path| {
                FirstMessageContentRequestPreparer {
                    llama_runtime: &runtime,
                    sequence_context_size,
                }
                .into_multimodal_generation_request_preparer(multimodal_projection_path)
            })?;
        let multimodal_ingestion = MultimodalIngestion::for_context(
            &llama_context,
            self.scheduler_model_capabilities(&runtime, &image_question_preparer)?,
        );
        let ConversationGeneration {
            agent_request,
            generate_tokens_stop_tx: _generate_tokens_stop_tx,
            mut generated_tokens_rx,
        } = ConversationGeneration::new(
            SingleImageQuestion {
                image_data_uri: ImageFixture::LlamasJpeg
                    .data_uri()
                    .context("the test image must load")?,
                max_tokens: NonZeroU32::MIN,
                question: IMAGE_QUESTION,
            }
            .into_conversation_params(),
            runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
        );
        let prepared_image_question = image_question_preparer
            .prepare_conversation_history(agent_request)
            .context("the image question must be prepared")?;
        let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

        scheduler_message_tx
            .send(SchedulerMessage::Command(prepared_image_question))
            .map_err(|send_error| undelivered_to_the_scheduler(&send_error))
            .and_then(|()| {
                scheduler_message_tx
                    .send(SchedulerMessage::Shutdown)
                    .map_err(|send_error| undelivered_to_the_scheduler(&send_error))
            })
            .context("the scheduler must accept the image question and its shutdown")?;

        ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
            agent_shutdown: CancellationToken::new(),
            batch: LlamaBatch::new(llama_context.limits()),
            llama_context,
            scheduler_context: ContinuousBatchSchedulerContext {
                agent_name: None,
                desired_slots_total: 1,
                model: &runtime.loaded_llama_model.model,
                multimodal_ingestion,
                sequence_context_size,
            },
            scheduler_message_rx,
        })
        .run();

        generated_tokens_rx
            .try_recv()
            .context("the scheduler must answer the image question")
    }

    fn scheduler_model_capabilities(
        &self,
        runtime: &LlamaRuntime,
        GenerationRequestPreparer {
            model_capabilities, ..
        }: &GenerationRequestPreparer,
    ) -> Result<Arc<EstablishedModelCapabilities>> {
        match self {
            Self::WithAMultimodalProjection => Ok(model_capabilities.clone()),
            Self::WithoutAMultimodalProjection => {
                runtime.establish_model_capabilities(default_model_capabilities_settings())
            }
        }
    }
}
