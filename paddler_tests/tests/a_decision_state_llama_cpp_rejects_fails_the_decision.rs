#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::DecodeError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::two_option_question::two_option_question;

use crate::kev_decision_model::KevDecisionModel;

#[test]
fn a_decision_state_llama_cpp_rejects_fails_the_decision() {
    let KevDecisionModel {
        delimiter_tokens,
        pointer_head,
        runtime,
    } = KevDecisionModel::load(1);
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
    let mut admitted_decision = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(2),
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    )
    .expect("the decision must be admitted");

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
                None,
            )
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(DecisionError::DecodeFailed(DecodeError::BatchInvalid).to_string())
    );
}
