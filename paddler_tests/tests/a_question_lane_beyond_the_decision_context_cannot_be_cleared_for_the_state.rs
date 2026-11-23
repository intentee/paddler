#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::error::memory_seq_rm_error::MemorySeqRmError;
use llama_cpp_bindings::error::sequence_id_out_of_range::SequenceIdOutOfRange;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;
use log::Level;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_agent_runtime::sequence_id::SequenceId;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::kev_decision_model::KevDecisionModel;
use paddler_tests::two_option_question::two_option_question;

const SEQUENCES_THE_CONTEXT_HOLDS: u32 = 1;
const SEQUENCES_THE_LEDGER_HANDS_OUT: u16 = 2;

#[test]
fn a_question_lane_beyond_the_decision_context_cannot_be_cleared_for_the_state() {
    CapturingLogger::install();

    let KevDecisionModel {
        delimiter_tokens,
        pointer_head,
        runtime,
    } = KevDecisionModel::load(1).expect("the kev decision model must load");
    let mut llama_context = runtime
        .create_decision_llama_context(SEQUENCES_THE_CONTEXT_HOLDS)
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(ContextLimits::of(&llama_context));
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
            },
            &pointer_head,
            None,
        )
        .expect("the decision must advance");

    let lane_clear_error = active_decision_request
        .advance(
            &mut DecisionDecoder {
                batch: &mut batch,
                llama_context: &mut llama_context,
            },
            &pointer_head,
            None,
        )
        .expect_err("the question lane beyond the context must not be cleared");

    assert_eq!(
        lane_clear_error.to_string(),
        DecisionError::KvCacheRemovalFailed(MemorySeqRmError::SequenceIdOutOfRange(
            SequenceIdOutOfRange {
                seq_id: 1,
                n_seq_max: SEQUENCES_THE_CONTEXT_HOLDS,
            }
        ))
        .to_string()
    );

    active_decision_request.finish_with(
        &mut DecisionDecoder {
            batch: &mut batch,
            llama_context: &mut llama_context,
        },
        None,
        lane_clear_error,
    );

    assert!(
        CapturingLogger::records_for("paddler_agent_runtime::sequence_id").contains(
            &CapturedRecord {
                level: Level::Error,
                message: format!(
                    "{:?}: {}; the next request admitted to sequence {:?} clears it again",
                    None::<&str>,
                    MemorySeqRmError::SequenceIdOutOfRange(SequenceIdOutOfRange {
                        seq_id: 1,
                        n_seq_max: SEQUENCES_THE_CONTEXT_HOLDS,
                    }),
                    SequenceId(1)
                ),
                target: "paddler_agent_runtime::sequence_id".to_owned(),
            }
        )
    );
}
