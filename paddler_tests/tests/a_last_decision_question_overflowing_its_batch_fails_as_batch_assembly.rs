#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;
use llama_cpp_bindings_types::BatchCapacity;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::two_option_question::two_option_question;

use crate::kev_decision_model::KevDecisionModel;

const TOKENS_THE_BATCH_HOLDS: NonZeroU16 = NonZeroU16::MIN;
const TOKENS_PER_QUESTION_CHUNK: usize = 2;

#[test]
fn a_last_decision_question_overflowing_its_batch_fails_as_batch_assembly() {
    let KevDecisionModel {
        delimiter_tokens,
        pointer_head,
        runtime,
    } = KevDecisionModel::load(1);
    let mut llama_context = runtime
        .create_decision_llama_context(1)
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(BatchCapacity::from(TOKENS_THE_BATCH_HOLDS), 1);
    let layout = DecisionTokenLayout::new(
        &delimiter_tokens,
        vec![LlamaToken::new(0)],
        Vec::new(),
        two_option_question("question"),
    );
    let state_tokens = layout.state.len();
    let AdmittedDecision {
        mut active_decision_request,
        decision_result_rx: _decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(1),
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    )
    .expect("the decision must be admitted");

    for _state_token in 0..state_tokens {
        active_decision_request
            .advance(
                &mut DecisionDecoder {
                    batch: &mut batch,
                    llama_context: &mut llama_context,
                    n_batch: TOKENS_THE_BATCH_HOLDS.get().into(),
                },
                &pointer_head,
                None,
            )
            .expect("the decision must advance");
    }

    assert_eq!(
        active_decision_request
            .advance(
                &mut DecisionDecoder {
                    batch: &mut batch,
                    llama_context: &mut llama_context,
                    n_batch: TOKENS_PER_QUESTION_CHUNK,
                },
                &pointer_head,
                None,
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
