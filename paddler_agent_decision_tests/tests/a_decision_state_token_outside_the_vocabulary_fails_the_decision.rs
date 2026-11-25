#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_context::context_error::ContextError;
use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_model::llama_token::LlamaToken;

use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_agent_decision_tests::admitted_decision::AdmittedDecision;
use paddler_agent_decision_tests::kev_decision_model::KevDecisionModel;
use paddler_agent_decision_tests::two_option_question::two_option_question;

#[test]
fn a_decision_state_token_outside_the_vocabulary_fails_the_decision() {
    let KevDecisionModel {
        delimiter_tokens,
        option_scorer,
        runtime,
    } = KevDecisionModel::load(1).expect("the kev decision model must load");
    let mut llama_context = runtime
        .create_decision_llama_context(2)
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());
    let n_vocab = runtime.loaded_llama_model.model.n_vocab();
    let layout = DecisionTokenLayout::new(
        &delimiter_tokens,
        vec![LlamaToken::new(n_vocab)],
        Vec::new(),
        two_option_question("question"),
    );
    let mut admitted_decision = AdmittedDecision::admit(
        &DecisionCapacityLedger::new(2),
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
        layout,
    )
    .expect("the decision must be admitted");

    assert!(matches!(
        admitted_decision.active_decision_request.advance(
            &mut DecisionDecoder {
                batch: &mut batch,
                llama_context: &mut llama_context,
            },
            &option_scorer,
            None,
        ),
        Err(DecisionError::BatchAssemblyFailed(
            ContextError::TokenOutsideVocabulary {
                token,
                n_vocab: reported_n_vocab,
            }
        )) if token == LlamaToken::new(n_vocab) && reported_n_vocab == n_vocab
    ));
}
