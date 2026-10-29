#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::ClearKvCacheSeqError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::two_option_question::two_option_question;

use crate::kev_decision_model::KevDecisionModel;

const SEQUENCES_THE_CONTEXT_HOLDS: u32 = 1;
const SEQUENCES_THE_LEDGER_HANDS_OUT: u16 = 2;

#[test]
fn a_question_lane_beyond_the_decision_context_cannot_be_cleared_for_the_state() {
    let KevDecisionModel {
        delimiter_tokens,
        pointer_head,
        runtime,
    } = KevDecisionModel::load(1);
    let mut llama_context = runtime
        .create_decision_llama_context(SEQUENCES_THE_CONTEXT_HOLDS)
        .expect("the decision context must be created");
    let n_batch = runtime.model_runtime_parameters.n_batch.tokens_usize();
    let mut batch = LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1);
    let layout = DecisionTokenLayout::new(
        &delimiter_tokens,
        vec![LlamaToken::new(0)],
        vec![two_option_question("leading")],
        two_option_question("last"),
    );
    let AdmittedDecision {
        mut active_decision_request,
        decision_result_rx: _decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(SEQUENCES_THE_LEDGER_HANDS_OUT),
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    )
    .expect("the decision must be admitted");

    active_decision_request
        .advance(
            &mut DecisionDecoder {
                batch: &mut batch,
                llama_context: &mut llama_context,
                n_batch,
            },
            &pointer_head,
            None,
        )
        .expect("the decision must advance");

    assert_eq!(
        active_decision_request
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
        Some(
            DecisionError::KvCacheRemovalFailed(ClearKvCacheSeqError::PartialSequenceNotRemoved {
                seq_id: 1,
                p0: -1,
                p1: -1,
            })
            .to_string()
        )
    );
}
