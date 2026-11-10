#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::chat_prompt_part::ChatPromptPart;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::error::eval_multimodal_chunks_error::EvalMultimodalChunksError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::mtmd::mtmd_eval_error::MtmdEvalError;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::media_marked_prompt::MediaMarkedPrompt;
use paddler_agent_text_generation::multimodal_prompt_ingestion::MultimodalPromptIngestion;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::prepared_prompt::PreparedPrompt;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::media_marker::MediaMarker;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

const PROMPT: &str = "Describe the picture";
const FITTED_MICRO_BATCH_TOKENS: u32 = 512;

fn ingestion_failure(
    eval_multimodal_chunks_error: EvalMultimodalChunksError,
) -> GeneratedTokenResult {
    GeneratedTokenResult::MultimodalIngestionFailed(
        TextGenerationError::MultimodalIngestionFailed(eval_multimodal_chunks_error).to_string(),
    )
}

#[test]
fn a_multimodal_prompt_that_cannot_be_ingested_ends_the_generation() {
    let first_result_of_a_multimodal_prompt =
        |adjust_ingestion: &dyn Fn(&mut MultimodalPromptIngestion)| -> GeneratedTokenResult {
            let runtime = LlamaRuntime::load(
                &smolvlm2_256m()
                    .cached_path()
                    .expect("the vision model must be in the Hugging Face cache"),
                agent_inference_runtime_context(1),
            )
            .expect("the vision model must load");
            let llama_context = runtime
                .create_text_generation_llama_context(1)
                .expect("the context must be created");
            let sequence_context_size = llama_context.n_ctx_seq();
            let mut ingestion = runtime
                .multimodal_prompt_support(
                    &llama_context,
                    &smolvlm2_256m_mmproj()
                        .cached_path()
                        .expect("the multimodal projection must be in the Hugging Face cache"),
                )
                .expect("the multimodal projection must load")
                .prepare_prompt(
                    &[],
                    &MediaMarkedPrompt {
                        media_marker: &MediaMarker::new("<media>".to_owned()),
                        prompt_parts: &[ChatPromptPart::Input(PROMPT.to_owned())],
                    },
                )
                .expect("the multimodal prompt must be prepared");

            adjust_ingestion(&mut ingestion);

            let RawPromptGeneration {
                agent_request,
                generate_tokens_stop_tx: _generate_tokens_stop_tx,
                mut generated_tokens_rx,
            } = RawPromptGeneration::new(
                PROMPT,
                runtime
                    .inference_runtime_context
                    .slot_aggregated_status
                    .clone(),
            );
            let request = FirstMessageContentRequestPreparer {
                llama_runtime: &runtime,
                sequence_context_size,
            }
            .into_generation_request_preparer()
            .expect("the first message content chat template must be prepared")
            .prepare_raw_prompt(agent_request)
            .expect("the generation must be prepared");
            let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

            scheduler_message_tx
                .send(SchedulerMessage::Command(PreparedGenerationRequest {
                    prompt: PreparedPrompt::Multimodal(ingestion),
                    ..request
                }))
                .expect("the scheduler must accept the generation");
            scheduler_message_tx
                .send(SchedulerMessage::Shutdown)
                .expect("the scheduler must accept the shutdown");

            ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
                agent_shutdown: CancellationToken::new(),
                batch: LlamaBatch::new(ContextLimits::of(&llama_context)),
                llama_context,
                scheduler_context: ContinuousBatchSchedulerContext {
                    agent_name: None,
                    desired_slots_total: 1,
                    model: &runtime.loaded_llama_model.model,
                    sequence_context_size,
                },
                scheduler_message_rx,
            })
            .run();

            generated_tokens_rx
                .try_recv()
                .expect("the scheduler must answer the generation")
        };

    assert_eq!(
        [
            first_result_of_a_multimodal_prompt(&|ingestion: &mut MultimodalPromptIngestion| {
                ingestion.next_chunk_index = ingestion.chunks.len();
            }),
            first_result_of_a_multimodal_prompt(&|ingestion: &mut MultimodalPromptIngestion| {
                ingestion.n_batch = i32::try_from(ingestion.chunks.micro_batch_tokens() - 1)
                    .expect("the fitted micro batch must fit a batch size");
            }),
        ],
        [
            ingestion_failure(EvalMultimodalChunksError::ChunkOutOfBounds(1)),
            ingestion_failure(EvalMultimodalChunksError::EvalFailed(
                MtmdEvalError::EvaluatedMicroBatchSmallerThanFitted {
                    evaluated_micro_batch_tokens: FITTED_MICRO_BATCH_TOKENS - 1,
                    fitted_micro_batch_tokens: FITTED_MICRO_BATCH_TOKENS,
                }
            )),
        ]
    );
}
