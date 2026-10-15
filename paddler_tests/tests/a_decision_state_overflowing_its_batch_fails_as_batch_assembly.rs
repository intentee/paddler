#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;
use llama_cpp_bindings_types::BatchCapacity;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_delimiter_tokens::DecisionDelimiterTokens;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_0_8b_pointer_head::kev_0_8b_pointer_head;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::two_option_question::two_option_question;

const TOKENS_THE_BATCH_HOLDS: NonZeroU16 = NonZeroU16::MIN;

#[test]
fn a_decision_state_overflowing_its_batch_fails_as_batch_assembly() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b_target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev decision model must load");
    let pointer_head = kev_0_8b_pointer_head().expect("the kev pointer head must load");
    let delimiter_tokens = DecisionDelimiterTokens::resolve(
        runtime.loaded_llama_model.model.as_ref(),
        &pointer_head.delimiters,
    )
    .expect("the kev delimiters must resolve");
    let mut llama_context = runtime
        .create_decision_llama_context(1)
        .expect("the decision context must be created");
    let n_batch = runtime.model_runtime_parameters.n_batch.tokens_usize();
    let mut batch = LlamaBatch::new(BatchCapacity::from(TOKENS_THE_BATCH_HOLDS), 1);
    let layout = DecisionTokenLayout::new(
        &delimiter_tokens,
        vec![LlamaToken::new(0)],
        Vec::new(),
        two_option_question("question"),
    );
    let admission = DecisionCapacityLedger::new(llama_context.n_ctx() as usize, 1)
        .admit(&layout.capacity_demand())
        .expect("the decision must be admitted");
    let AdmittedDecision {
        mut active_decision_request,
        decision_result_rx: _decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::new(
        admission,
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    );

    assert_eq!(
        active_decision_request
            .advance(
                &mut DecisionDecoder {
                    batch: &mut batch,
                    llama_context: &mut llama_context,
                    n_batch,
                },
                &pointer_head,
            )
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            DecisionError::BatchAssemblyFailed(BatchAddError::InsufficientSpace(usize::from(
                TOKENS_THE_BATCH_HOLDS.get()
            )))
            .to_string()
        )
    );
}
