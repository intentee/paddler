#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;
use paddler_tests::admitted_decision::AdmittedDecision;
use paddler_tests::two_option_question::two_option_question;

use crate::kev_decision_model::KevDecisionModel;

pub struct KevDecisionInAKvCache {
    pub cache_cells: NonZeroU32,
    pub state_tokens_after_the_delimiter: usize,
}

impl KevDecisionInAKvCache {
    pub fn first_two_advances(&self) -> Vec<Result<DecisionAdvance, String>> {
        let KevDecisionModel {
            delimiter_tokens,
            pointer_head,
            mut runtime,
        } = KevDecisionModel::load(1);

        runtime.model_runtime_parameters.context_size = self.cache_cells;

        let mut llama_context = runtime
            .create_decision_llama_context(1)
            .expect("the decision context must be created");
        let mut batch = LlamaBatch::new(ContextLimits::of(&llama_context));
        let AdmittedDecision {
            mut active_decision_request,
            decision_result_rx: _decision_result_rx,
            decision_stop_tx: _decision_stop_tx,
        } = AdmittedDecision::admit(
            &DecisionCapacityLedger::new(1),
            runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
            DecisionTokenLayout::new(
                &delimiter_tokens,
                vec![LlamaToken::new(0); self.state_tokens_after_the_delimiter],
                Vec::new(),
                two_option_question("question"),
            ),
        )
        .expect("the decision must be admitted");

        (0..2)
            .map(|_advance_index| {
                active_decision_request
                    .advance(
                        &mut DecisionDecoder {
                            batch: &mut batch,
                            llama_context: &mut llama_context,
                        },
                        &pointer_head,
                        None,
                    )
                    .map_err(|decision_error| decision_error.to_string())
            })
            .collect()
    }
}
