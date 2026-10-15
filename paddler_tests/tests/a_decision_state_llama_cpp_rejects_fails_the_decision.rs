#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::DecodeError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

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

#[test]
fn a_decision_state_llama_cpp_rejects_fails_the_decision() {
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
        .create_decision_llama_context(2)
        .expect("the decision context must be created");
    let n_batch = runtime.model_runtime_parameters.n_batch.tokens_usize();
    let mut batch = LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1);
    let layout = DecisionTokenLayout::new(
        &delimiter_tokens,
        vec![LlamaToken::new(runtime.loaded_llama_model.model.n_vocab())],
        Vec::new(),
        two_option_question("question"),
    );
    let admission = DecisionCapacityLedger::new(llama_context.n_ctx() as usize, 2)
        .admit(&layout.capacity_demand())
        .expect("the decision must be admitted");
    let mut admitted_decision = AdmittedDecision::new(
        admission,
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    );

    assert_eq!(
        admitted_decision
            .active_decision_request
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
        Some(DecisionError::DecodeFailed(DecodeError::BatchInvalid).to_string())
    );
}
