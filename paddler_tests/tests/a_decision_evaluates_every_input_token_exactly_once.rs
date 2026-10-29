#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::two_option_question::two_option_question;

use crate::kev_decision_model::KevDecisionModel;

#[test]
fn a_decision_evaluates_every_input_token_exactly_once() {
    let KevDecisionModel {
        delimiter_tokens,
        pointer_head,
        runtime,
    } = KevDecisionModel::load(DECISION_SLOTS_MINIMUM.get());
    let mut llama_context = runtime
        .create_decision_llama_context(u32::from(DECISION_SLOTS_MINIMUM.get()))
        .expect("the decision context must be created");
    let n_batch = runtime.model_runtime_parameters.n_batch.tokens_usize();
    let mut batch = LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1);
    let layout = DecisionTokenLayout::new(
        &delimiter_tokens,
        vec![LlamaToken::new(0); 8],
        vec![two_option_question("leading")],
        two_option_question("last"),
    );
    let decision_input_tokens = layout.total_tokens();
    let AdmittedDecision {
        mut active_decision_request,
        decision_result_rx: _decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(DECISION_SLOTS_MINIMUM.get()),
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    )
    .expect("the decision must be admitted");

    llama_context.reset_timings();

    while matches!(
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
            .expect("the decision must advance"),
        DecisionAdvance::Continuing
    ) {}

    llama_context.synchronize();

    assert_eq!(
        usize::try_from(llama_context.timings().n_p_eval())
            .expect("the evaluated token count must not be negative"),
        decision_input_tokens
    );
}
