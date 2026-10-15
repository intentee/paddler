#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;
use std::sync::mpsc::channel;

use hf_hub::Cache;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdContextParams;
use llama_cpp_bindings::mtmd::micro_batch_tokens;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::json_tools_serializer::JsonToolsSerializer;
use paddler_agent_text_generation::multimodal_prompt_ingestion::MultimodalPromptIngestion;
use paddler_agent_text_generation::multimodal_prompt_support::MultimodalPromptSupport;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::prepared_prompt::PreparedPrompt;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;
use paddler_tests::text_only_generation_parts::TextOnlyGenerationParts;

#[test]
fn a_multimodal_prompt_that_cannot_be_ingested_ends_the_generation() {
    let first_result_of_a_multimodal_prompt =
        |adjust_ingestion: &dyn Fn(&mut MultimodalPromptIngestion)| -> GeneratedTokenResult {
            let runtime = LlamaRuntime::load(
                &smolvlm2_256m()
                    .reference
                    .cached_path(&Cache::from_env())
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
                add_special_tokens: true,
                micro_batch_tokens: micro_batch_tokens(&llama_context, n_batch.tokens()),
                multimodal_context: Arc::new(
                    MtmdContext::init_from_file(
                        &smolvlm2_256m_mmproj()
                            .reference
                            .cached_path(&Cache::from_env())
                            .expect("the multimodal projection must be in the Hugging Face cache")
                            .to_string_lossy(),
                        &runtime.loaded_llama_model.model,
                        &MtmdContextParams::default(),
                    )
                    .expect("the multimodal projection must load"),
                ),
                n_batch: n_batch.tokens_i32(),
                sequence_context_size,
            }
            .prepare_prompt(&[], "Describe the picture".to_owned())
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
            let request = runtime
                .text_only_generation_request_preparer(
                    TextOnlyGenerationParts {
                        chat_template_renderer: ChatTemplateRenderer::new(
                            first_message_content_chat_template(),
                        )
                        .expect("the chat template must compile"),
                        sequence_context_size,
                        streaming_markers: runtime
                            .loaded_llama_model
                            .model
                            .streaming_markers()
                            .expect("the streaming markers must be detected"),
                    },
                    runtime.sampler_chain_factory(),
                    JsonToolsSerializer,
                )
                .prepare_raw_prompt(agent_request)
                .expect("the generation must be prepared");
            let (scheduler_message_tx, scheduler_message_rx) = channel();

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
                    model: runtime.loaded_llama_model.model.clone(),
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
