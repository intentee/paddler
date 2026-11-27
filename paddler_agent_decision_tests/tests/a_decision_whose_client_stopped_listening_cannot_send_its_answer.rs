#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_context::llama_batch::LlamaBatch;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision_tests::admitted_decision::AdmittedDecision;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::many_question_decision::many_question_decision;

#[test]
fn a_decision_whose_client_stopped_listening_cannot_send_its_answer() {
    let decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), 1).expect("the kev decision model must load");
    let mut llama_context = decision_model_runtime
        .create_llama_context(1)
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());
    let AdmittedDecision {
        mut active_decision_request,
        decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(1),
        decision_model_runtime
            .prepared_decision(many_question_decision(1), NonZeroU16::MIN, u32::MAX)
            .expect("the decision must be prepared"),
    )
    .expect("the decision must be admitted");

    drop(decision_result_rx);

    let first_failure = loop {
        match active_decision_request.advance(
            &mut DecisionDecoder {
                batch: &mut batch,
                llama_context: &mut llama_context,
            },
            None,
            None,
        ) {
            Ok(DecisionAdvance::Continuing) => {}
            outcome => break outcome,
        }
    };

    assert!(matches!(
        first_failure,
        Err(DecisionError::ClientDisconnected(_))
    ));
}
