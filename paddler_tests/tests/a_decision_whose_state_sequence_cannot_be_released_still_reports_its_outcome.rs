#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::two_option_question::two_option_question;

use crate::kev_decision_model::KevDecisionModel;

const SEQUENCES_THE_CONTEXT_HOLDS: u32 = 1;
const SEQUENCES_THE_LEDGER_HANDS_OUT: u16 = 4;

#[test]
fn a_decision_whose_state_sequence_cannot_be_released_still_reports_its_outcome() {
    let KevDecisionModel {
        delimiter_tokens,
        runtime,
        ..
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
    let capacity_ledger = DecisionCapacityLedger::new(SEQUENCES_THE_LEDGER_HANDS_OUT);
    let _decision_holding_the_sequences_the_context_holds = AdmittedDecision::admit(
        &capacity_ledger,
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout.clone(),
    )
    .expect("the first decision must be admitted");
    let AdmittedDecision {
        active_decision_request,
        mut decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &capacity_ledger,
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    )
    .expect("the second decision must be admitted");

    active_decision_request.finish_with(
        &mut DecisionDecoder {
            batch: &mut batch,
            llama_context: &mut llama_context,
            n_batch,
        },
        None,
        DecisionResult::StopRequested,
    );

    assert_eq!(
        decision_result_rx.try_recv(),
        Ok(DecisionResult::StopRequested)
    );
}
