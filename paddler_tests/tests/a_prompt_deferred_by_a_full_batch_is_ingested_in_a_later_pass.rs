#![cfg(feature = "tests_that_use_llms")]

use std::mem::Discriminant;
use std::mem::discriminant;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::ParseSpecialTokens;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::generation_done_discriminant::generation_done_discriminant;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

const SENTENCE: &str = "Paddler balances inference across agents. ";
const SLOTS: u16 = 2;

fn last_result_discriminant(
    mut generated_tokens_rx: UnboundedReceiver<GeneratedTokenResult>,
) -> Option<Discriminant<GeneratedTokenResult>> {
    let mut last_result = None;

    while let Ok(generated_token_result) = generated_tokens_rx.try_recv() {
        last_result = Some(discriminant(&generated_token_result));
    }

    last_result
}

#[test]
fn a_prompt_deferred_by_a_full_batch_is_ingested_in_a_later_pass() {
    let runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the kev model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(u32::from(SLOTS))
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let n_batch = runtime.model_runtime_parameters.n_batch;
    let model = &runtime.loaded_llama_model.model;
    let tokens_of_repeated_sentence = |repeats: usize| {
        model
            .str_to_token(
                &SENTENCE.repeat(repeats),
                AddBos::Never,
                ParseSpecialTokens::Always,
            )
            .expect("the repeated sentence must tokenize")
            .len()
    };
    let tokens_per_repeated_sentence =
        tokens_of_repeated_sentence(2) - tokens_of_repeated_sentence(1);
    let batch_filling_prompt =
        SENTENCE.repeat(n_batch.tokens_usize() / tokens_per_repeated_sentence + 1);
    let batch_filling_prompt_tokens = model
        .str_to_token(
            &batch_filling_prompt,
            AddBos::Always,
            ParseSpecialTokens::Always,
        )
        .expect("the batch filling prompt must tokenize")
        .len();

    assert!(
        batch_filling_prompt_tokens > n_batch.tokens_usize()
            && batch_filling_prompt_tokens < sequence_context_size as usize,
        "{batch_filling_prompt_tokens}"
    );

    let request_preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size,
    }
    .into_generation_request_preparer();
    let RawPromptGeneration {
        agent_request: batch_filling_generation,
        generate_tokens_stop_tx: _batch_filling_generate_tokens_stop_tx,
        generated_tokens_rx: batch_filling_generated_tokens_rx,
    } = RawPromptGeneration::new(
        &batch_filling_prompt,
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );
    let RawPromptGeneration {
        agent_request: deferred_generation,
        generate_tokens_stop_tx: _deferred_generate_tokens_stop_tx,
        generated_tokens_rx: deferred_generated_tokens_rx,
    } = RawPromptGeneration::new(
        "Paddler",
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for agent_request in [batch_filling_generation, deferred_generation] {
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
        batch: LlamaBatch::new(n_batch.capacity(), i32::from(SLOTS)),
        llama_context,
        scheduler_context: ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: SLOTS,
            model: &runtime.loaded_llama_model.model,
            n_batch,
            sequence_context_size,
        },
        scheduler_message_rx,
    })
    .run();

    assert_eq!(
        [
            last_result_discriminant(batch_filling_generated_tokens_rx),
            last_result_discriminant(deferred_generated_tokens_rx),
        ],
        [
            Some(generation_done_discriminant()),
            Some(generation_done_discriminant())
        ]
    );
}
