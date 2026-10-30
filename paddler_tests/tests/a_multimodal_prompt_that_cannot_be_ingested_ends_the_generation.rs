#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::mtmd::micro_batch_tokens;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::media_marked_prompt::MediaMarkedPrompt;
use paddler_agent_text_generation::model_bos_token::ModelBosToken;
use paddler_agent_text_generation::multimodal_prompt_ingestion::MultimodalPromptIngestion;
use paddler_agent_text_generation::multimodal_prompt_support::MultimodalPromptSupport;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::prepared_prompt::PreparedPrompt;
use paddler_agent_text_generation::resident_multimodal_context::ResidentMultimodalContext;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::media_marker::MediaMarker;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

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
            let n_batch = runtime.model_runtime_parameters.n_batch;
            let mut ingestion = MultimodalPromptSupport {
                micro_batch_tokens: micro_batch_tokens(&llama_context, n_batch.tokens()),
                model_bos_token: ModelBosToken::of(&runtime.loaded_llama_model.model),
                multimodal_context: Arc::new(
                    ResidentMultimodalContext::load(
                        &smolvlm2_256m_mmproj()
                            .cached_path()
                            .expect("the multimodal projection must be in the Hugging Face cache"),
                        &runtime.loaded_llama_model.model,
                        runtime
                            .inference_runtime_context
                            .vram_residencies
                            .multimodal_projection
                            .try_occupy()
                            .expect("no other multimodal projection may be resident"),
                    )
                    .expect("the multimodal projection must load"),
                ),
                n_batch: n_batch.tokens_i32(),
                sequence_context_size,
            }
            .prepare_prompt(
                &[],
                &MediaMarkedPrompt {
                    media_marker: &MediaMarker::new("<media>".to_owned()),
                    prompt: "Describe the picture",
                },
            )
            .expect("the multimodal prompt must be prepared");

            adjust_ingestion(&mut ingestion);

            let RawPromptGeneration {
                agent_request,
                generate_tokens_stop_tx: _generate_tokens_stop_tx,
                mut generated_tokens_rx,
            } = RawPromptGeneration::new(
                "Describe the picture",
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
                batch: LlamaBatch::new(n_batch.capacity(), 1),
                llama_context,
                scheduler_context: ContinuousBatchSchedulerContext {
                    agent_name: None,
                    desired_slots_total: 1,
                    model: &runtime.loaded_llama_model.model,
                    n_batch,
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
            GeneratedTokenResult::MultimodalIngestionFailed(
                "failed to ingest multimodal prompt: chunk index 1 out of bounds during post-eval walk"
                    .to_owned()
            ),
            GeneratedTokenResult::MultimodalIngestionFailed(
                "failed to ingest multimodal prompt: a chunk fitted to decodes of 512 tokens cannot be evaluated in decodes of 511"
                    .to_owned()
            ),
        ]
    );
}
