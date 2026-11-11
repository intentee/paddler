use std::num::NonZeroU32;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_token_layout::DecisionTokenLayout;

use crate::admitted_decision::AdmittedDecision;
use crate::kev_decision_model::KevDecisionModel;
use crate::two_option_question::two_option_question;

pub struct KevDecisionInAKvCache {
    pub cache_cells: NonZeroU32,
    pub state_tokens_after_the_delimiter: usize,
}

impl KevDecisionInAKvCache {
    /// # Errors
    ///
    /// Returns an error when the kev decision model cannot load, when its decision context cannot be created, and when the decision is not admitted.
    pub fn first_two_advances(&self) -> Result<Vec<Result<DecisionAdvance, String>>> {
        KevDecisionModel::load(1)
            .and_then(|kev_decision_model| self.advance_twice(kev_decision_model))
    }

    fn advance_twice(
        &self,
        KevDecisionModel {
            delimiter_tokens,
            pointer_head,
            mut runtime,
        }: KevDecisionModel,
    ) -> Result<Vec<Result<DecisionAdvance, String>>> {
        runtime.model_runtime_parameters.context_size = self.cache_cells;

        runtime
            .create_decision_llama_context(1)
            .context("the decision context must be created")
            .and_then(|mut llama_context| {
                let mut batch = LlamaBatch::new(ContextLimits::of(&llama_context));

                AdmittedDecision::admit(
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
                .context("the decision must be admitted")
                .map(
                    |AdmittedDecision {
                         mut active_decision_request,
                         decision_result_rx: _decision_result_rx,
                         decision_stop_tx: _decision_stop_tx,
                     }| {
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
                    },
                )
            })
    }
}
