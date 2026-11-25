#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_context_memory::context_memory_error::ContextMemoryError;
use log::Level;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision_tests::admitted_decision::AdmittedDecision;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::sample_decision::sample_decision;

const SEQUENCES_THE_CONTEXT_HOLDS: u32 = 1;
const SEQUENCES_THE_LEDGER_HANDS_OUT: u16 = 2;

#[test]
fn a_question_lane_beyond_the_decision_context_cannot_be_cleared_for_the_state() {
    CapturingLogger::install();

    let decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), 1).expect("the kev decision model must load");
    let mut llama_context = decision_model_runtime
        .create_llama_context(SEQUENCES_THE_CONTEXT_HOLDS)
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());
    let AdmittedDecision {
        mut active_decision_request,
        decision_result_rx: _decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(SEQUENCES_THE_LEDGER_HANDS_OUT),
        decision_model_runtime
            .prepared_decision(sample_decision(), DECISION_SLOTS_MINIMUM, u32::MAX)
            .expect("the decision must be prepared"),
    )
    .expect("the decision must be admitted");
    let lane_clear_error = loop {
        match active_decision_request.advance(
            &mut DecisionDecoder {
                batch: &mut batch,
                llama_context: &mut llama_context,
            },
            None,
            None,
        ) {
            Ok(decision_advance) => assert_eq!(
                decision_advance,
                DecisionAdvance::Continuing,
                "the decision must not complete without its question lane"
            ),
            Err(advance_error) => break advance_error,
        }
    };

    assert!(matches!(
        lane_clear_error,
        DecisionError::KvCacheRemovalFailed(ContextMemoryError::SeqRmSequenceIdOutOfRange {
            seq_id: 1,
            n_seq_max: SEQUENCES_THE_CONTEXT_HOLDS,
        })
    ));

    active_decision_request.finish_with(
        &mut DecisionDecoder {
            batch: &mut batch,
            llama_context: &mut llama_context,
        },
        None,
        lane_clear_error,
    );

    assert!(
        CapturingLogger::records_for("paddler_agent_runtime::sequence_id")
            .iter()
            .any(|CapturedRecord { level, .. }| *level == Level::Error)
    );
}
