#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::ClearKvCacheSeqError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

const SEQUENCES_THE_CONTEXT_HOLDS: u32 = 1;
const SEQUENCES_THE_SCHEDULER_HANDS_OUT: u16 = 2;

#[test]
fn a_generation_admitted_to_a_sequence_that_cannot_be_cleared_fails_with_kv_cache_clear() {
    let runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(SEQUENCES_THE_SCHEDULER_HANDS_OUT),
    )
    .expect("the kev model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(SEQUENCES_THE_CONTEXT_HOLDS)
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let request_preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size,
    }
    .into_generation_request_preparer();
    let raw_prompt_generation = || {
        RawPromptGeneration::new(
            "Paddler",
            runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
        )
    };
    let RawPromptGeneration {
        agent_request: generation_on_a_clearable_sequence,
        generate_tokens_stop_tx: _clearable_generate_tokens_stop_tx,
        generated_tokens_rx: _clearable_generated_tokens_rx,
    } = raw_prompt_generation();
    let RawPromptGeneration {
        agent_request: generation_on_a_sequence_beyond_the_context,
        generate_tokens_stop_tx: _beyond_generate_tokens_stop_tx,
        mut generated_tokens_rx,
    } = raw_prompt_generation();
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for agent_request in [
        generation_on_a_clearable_sequence,
        generation_on_a_sequence_beyond_the_context,
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
            desired_slots_total: SEQUENCES_THE_SCHEDULER_HANDS_OUT,
            model: &runtime.loaded_llama_model.model,
            n_batch: runtime.model_runtime_parameters.n_batch,
            sequence_context_size,
        },
        scheduler_message_rx,
    })
    .run();

    assert_eq!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::KvCacheClearFailed(
            TextGenerationError::KvCacheClearFailed(
                ClearKvCacheSeqError::PartialSequenceNotRemoved {
                    seq_id: 1,
                    p0: -1,
                    p1: -1,
                }
            )
            .to_string()
        ))
    );
}
