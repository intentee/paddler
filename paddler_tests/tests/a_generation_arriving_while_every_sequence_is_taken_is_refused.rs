#![cfg(feature = "tests_that_use_llms")]

use std::sync::mpsc::channel;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::json_tools_serializer::JsonToolsSerializer;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;
use paddler_tests::text_only_generation_parts::TextOnlyGenerationParts;

#[test]
fn a_generation_arriving_while_every_sequence_is_taken_is_refused() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b_target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let request_preparer = runtime.text_only_generation_request_preparer(
        TextOnlyGenerationParts {
            chat_template_renderer:
                ChatTemplateRenderer::new(first_message_content_chat_template())
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
    );
    let RawPromptGeneration {
        agent_request: generation_taking_the_only_sequence,
        generate_tokens_stop_tx: _first_generate_tokens_stop_tx,
        generated_tokens_rx: _first_generated_tokens_rx,
    } = RawPromptGeneration::new(
        "Paddler",
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );
    let RawPromptGeneration {
        agent_request: generation_without_a_sequence,
        generate_tokens_stop_tx: _second_generate_tokens_stop_tx,
        mut generated_tokens_rx,
    } = RawPromptGeneration::new(
        "Paddler",
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );
    let (scheduler_message_tx, scheduler_message_rx) = channel();

    for agent_request in [
        generation_taking_the_only_sequence,
        generation_without_a_sequence,
    ] {
        scheduler_message_tx
            .send(SchedulerMessage::Command(
                request_preparer
                    .prepare_raw_prompt(agent_request)
                    .expect("the generation must be prepared"),
            ))
            .expect("the scheduler must accept the generation");
    }
    scheduler_message_tx
        .send(SchedulerMessage::Shutdown)
        .expect("the scheduler must accept the shutdown");

    ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
        agent_shutdown: CancellationToken::new(),
        batch: LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1),
        llama_context,
        scheduler_context: ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: 1,
            model: runtime.loaded_llama_model.model.clone(),
            n_batch: runtime.model_runtime_parameters.n_batch,
            sequence_context_size,
        },
        scheduler_message_rx,
    })
    .run();

    assert_eq!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::NoSequenceSlotAvailable(
            TextGenerationError::NoSequenceSlotAvailable.to_string()
        ))
    );
}
