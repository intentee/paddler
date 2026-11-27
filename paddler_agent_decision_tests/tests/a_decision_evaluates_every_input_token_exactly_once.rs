#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_context::llama_batch::LlamaBatch;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::planned_decision_prompt::PlannedDecisionPrompt;
use paddler_agent_decision_tests::admitted_decision::AdmittedDecision;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_messaging::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::sample_decision::sample_decision;

#[test]
fn a_decision_evaluates_every_input_token_exactly_once() {
    let decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), DECISION_SLOTS_WITH_A_QUESTION_LANE.get())
            .expect("the kev decision model must load");
    let prepared_decision = decision_model_runtime
        .prepared_decision(
            sample_decision(),
            DECISION_SLOTS_WITH_A_QUESTION_LANE,
            u32::MAX,
        )
        .expect("the decision must be prepared");
    let plan = &prepared_decision.request.plan;
    let shared_prefix_length = plan.shared_prefix_length;
    let every_input_token_once = plan.prompts.iter().fold(
        shared_prefix_length,
        |tokens, PlannedDecisionPrompt { prompt, .. }| {
            tokens + prompt.tokens().len() - shared_prefix_length
        },
    );
    let mut llama_context = decision_model_runtime
        .create_llama_context(u32::from(DECISION_SLOTS_WITH_A_QUESTION_LANE.get()))
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());
    let AdmittedDecision {
        mut active_decision_request,
        decision_result_rx: _decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(DECISION_SLOTS_WITH_A_QUESTION_LANE.get()),
        prepared_decision,
    )
    .expect("the decision must be admitted");
    let tokens_evaluated_before_the_decision = llama_context.evaluated_tokens();

    assert!(shared_prefix_length > 0);

    while matches!(
        active_decision_request
            .advance(
                &mut DecisionDecoder {
                    batch: &mut batch,
                    llama_context: &mut llama_context,
                },
                None,
                None,
            )
            .expect("the decision must advance"),
        DecisionAdvance::Continuing
    ) {}

    assert_eq!(
        usize::try_from(llama_context.evaluated_tokens() - tokens_evaluated_before_the_decision)
            .expect("the evaluated token count must fit in memory"),
        every_input_token_once
    );
    assert!(matches!(
        active_decision_request.summary(),
        DecisionResult::Done(DecisionSummary { input_tokens, .. })
            if input_tokens == every_input_token_once
    ));
}
