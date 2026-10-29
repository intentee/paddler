#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;
use tokio::sync::mpsc::error::SendError;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_messaging::decision_result::DecisionResult;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::two_option_question::two_option_question;

use crate::kev_decision_model::KevDecisionModel;

#[test]
fn a_decision_whose_client_stopped_listening_cannot_send_its_answer() {
    let KevDecisionModel {
        delimiter_tokens,
        pointer_head,
        runtime,
    } = KevDecisionModel::load(1);
    let mut llama_context = runtime
        .create_decision_llama_context(1)
        .expect("the decision context must be created");
    let n_batch = runtime.model_runtime_parameters.n_batch.tokens_usize();
    let mut batch = LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1);
    let layout = DecisionTokenLayout::new(
        &delimiter_tokens,
        vec![LlamaToken::new(0)],
        Vec::new(),
        two_option_question("question"),
    );
    let AdmittedDecision {
        mut active_decision_request,
        decision_result_rx,
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

    drop(decision_result_rx);

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
            DecisionError::ClientDisconnected(SendError(DecisionResult::StopRequested)).to_string()
        )
    );
}
