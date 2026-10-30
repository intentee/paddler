#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::num::NonZeroU32;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::generation_done_discriminant::generation_done_discriminant;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

const PROMPT: &str = "Paddler balances inference across agents, and";
const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(8).unwrap();

#[test]
fn a_generation_evaluates_its_prompt_and_each_generated_token_exactly_once() {
    let runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let mut llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let n_batch = runtime.model_runtime_parameters.n_batch;
    let request_preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size,
    }
    .into_generation_request_preparer();
    let prompt_tokens = request_preparer
        .prompt_tokenizer
        .tokenize(PROMPT)
        .expect("the prompt must tokenize")
        .len();
    let RawPromptGeneration {
        mut agent_request,
        generate_tokens_stop_tx: _generate_tokens_stop_tx,
        mut generated_tokens_rx,
    } = RawPromptGeneration::new(
        PROMPT,
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );

    agent_request.params.max_tokens = MAX_TOKENS;

    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    scheduler_message_tx
        .send(SchedulerMessage::Command(
            request_preparer
                .prepare_raw_prompt(agent_request)
                .expect("the generation must be prepared"),
        ))
        .expect("the scheduler must accept the generation");
    scheduler_message_tx
        .send(SchedulerMessage::Shutdown)
        .expect("the scheduler must accept the shutdown");

    llama_context.reset_timings();

    let mut scheduler = ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
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
    });

    scheduler.run();

    let mut generated_token_results = Vec::new();

    while let Ok(generated_token_result) = generated_tokens_rx.try_recv() {
        generated_token_results.push(discriminant(&generated_token_result));
    }

    let timings = scheduler.llama_context.timings();

    assert_eq!(
        generated_token_results.last(),
        Some(&generation_done_discriminant())
    );
    assert_eq!(
        usize::try_from(timings.n_p_eval()).expect("the evaluated prompt tokens must count up"),
        prompt_tokens
    );
    assert_eq!(
        usize::try_from(timings.n_eval()).expect("the evaluated generated tokens must count up"),
        generated_token_results.len() - 2
    );
}
