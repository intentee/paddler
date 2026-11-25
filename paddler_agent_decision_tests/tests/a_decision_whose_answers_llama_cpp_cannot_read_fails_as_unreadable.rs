#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_bindings_sys::bindings::LLAMA_RS_FAULT_POINT_DECISION_ANSWERS_READ_THROWS;
use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_fault_injection::armed_fault::ArmedFault;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision_tests::admitted_decision::AdmittedDecision;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_agent_decision_tests::snowy_weather_decision::snowy_weather_decision;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

#[test]
fn a_decision_whose_answers_llama_cpp_cannot_read_fails_as_unreadable() {
    let decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), 1).expect("the kev decision model must load");
    let mut llama_context = decision_model_runtime
        .create_llama_context(1)
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());
    let mut admitted_decision = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(1),
        decision_model_runtime
            .prepared_decision(snowy_weather_decision(), NonZeroU16::MIN, u32::MAX)
            .expect("the decision must be prepared"),
    )
    .expect("the decision must be admitted");
    let _answers_unreadable = ArmedFault::at(LLAMA_RS_FAULT_POINT_DECISION_ANSWERS_READ_THROWS);

    assert!(matches!(
        admitted_decision.advance_until_it_ends(&mut DecisionDecoder {
            batch: &mut batch,
            llama_context: &mut llama_context,
        }),
        Err(DecisionError::AnswersUnreadable(_))
    ));
}
