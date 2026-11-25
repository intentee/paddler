#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_context::llama_batch::LlamaBatch;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision_tests::admitted_decision::AdmittedDecision;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_result::DecisionResult;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::sample_decision::sample_decision;

const SEQUENCES_THE_CONTEXT_HOLDS: u32 = 1;
const SEQUENCES_THE_LEDGER_HANDS_OUT: u16 = 4;

#[test]
fn a_decision_whose_state_sequence_cannot_be_released_still_reports_its_outcome() {
    let decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), 1).expect("the kev decision model must load");
    let mut llama_context = decision_model_runtime
        .create_llama_context(SEQUENCES_THE_CONTEXT_HOLDS)
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());
    let capacity_ledger = DecisionCapacityLedger::new(SEQUENCES_THE_LEDGER_HANDS_OUT);
    let prepared_decision = || {
        decision_model_runtime
            .prepared_decision(sample_decision(), DECISION_SLOTS_MINIMUM, u32::MAX)
            .expect("the decision must be prepared")
    };
    let _decision_holding_the_sequences_the_context_holds =
        AdmittedDecision::admit(&capacity_ledger, prepared_decision())
            .expect("the first decision must be admitted");
    let AdmittedDecision {
        active_decision_request,
        mut decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(&capacity_ledger, prepared_decision())
        .expect("the second decision must be admitted");

    active_decision_request.finish_with(
        &mut DecisionDecoder {
            batch: &mut batch,
            llama_context: &mut llama_context,
        },
        None,
        DecisionResult::StopRequested,
    );

    assert_eq!(
        decision_result_rx.try_recv(),
        Ok(DecisionResult::StopRequested)
    );
}
