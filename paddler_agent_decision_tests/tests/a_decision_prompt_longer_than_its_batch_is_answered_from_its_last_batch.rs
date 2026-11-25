#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_context::llama_batch::LlamaBatch;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision_tests::admitted_decision::AdmittedDecision;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_result::DecisionResult;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::sample_decision::sample_decision;

#[test]
fn a_decision_prompt_longer_than_its_batch_is_answered_from_its_last_batch() {
    let mut decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), DECISION_SLOTS_MINIMUM.get())
            .expect("the kev decision model must load");
    let prompts = decision_model_runtime
        .prepared_decision(sample_decision(), DECISION_SLOTS_MINIMUM, u32::MAX)
        .expect("the decision must be prepared")
        .request
        .plan
        .prompts;
    let longest_read_span = prompts
        .iter()
        .map(|prompt| prompt.tokens().len() - prompt.first_read_position())
        .fold(0, usize::max);
    let longest_prompt = prompts
        .iter()
        .map(|prompt| prompt.tokens().len())
        .fold(0, usize::max);

    assert!(longest_read_span < longest_prompt);

    decision_model_runtime
        .runtime
        .model_runtime_parameters
        .n_batch = ModelDefaultOr::Explicit(
        BatchSize::try_from(
            u32::try_from(longest_read_span).expect("the read span must fit a batch size"),
        )
        .expect("the read span must be a valid batch size"),
    );

    let mut llama_context = decision_model_runtime
        .create_llama_context(u32::from(DECISION_SLOTS_MINIMUM.get()))
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());
    let AdmittedDecision {
        mut active_decision_request,
        mut decision_result_rx,
        decision_stop_tx: _decision_stop_tx,
    } = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(DECISION_SLOTS_MINIMUM.get()),
        decision_model_runtime
            .prepared_decision(sample_decision(), DECISION_SLOTS_MINIMUM, u32::MAX)
            .expect("the decision must fit the batch holding its read span"),
    )
    .expect("the decision must be admitted");

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

    let mut answered_questions = Vec::new();

    while let Ok(DecisionResult::QuestionAnswered(answered_question)) =
        decision_result_rx.try_recv()
    {
        answered_questions.push(answered_question.id);
    }

    assert_eq!(
        answered_questions,
        sample_decision()
            .questions
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    );
}
