#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_context::llama_batch::LlamaBatch;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::multimodal_ingestion::MultimodalIngestion;
use paddler_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

#[test]
fn a_generation_arriving_while_every_sequence_is_taken_is_refused() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let request_preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size,
    }
    .into_generation_request_preparer()
    .expect("the first message content chat template must be prepared");
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
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

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
        batch: LlamaBatch::new(llama_context.limits()),
        llama_context,
        scheduler_context: ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: 1,
            model: &runtime.loaded_llama_model.model,
            multimodal_ingestion: MultimodalIngestion::Unsupported,
            sequence_context_size,
        },
        scheduler_message_rx,
    })
    .run();

    assert!(matches!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::NoSequenceSlotAvailable(_))
    ));
}
