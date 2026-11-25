use std::num::NonZeroU16;
use std::num::NonZeroU32;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_context::llama_batch::LlamaBatch;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_capacity_ledger::DecisionCapacityLedger;
use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_request_params_fixture::repeated_state_decision::repeated_state_decision;

use crate::admitted_decision::AdmittedDecision;
use crate::decision_model_runtime::DecisionModelRuntime;

pub struct DecisionInASmallKvCache {
    pub cache_cells: NonZeroU32,
    pub question_count: usize,
    pub state_repetitions: usize,
}

impl DecisionInASmallKvCache {
    /// # Errors
    ///
    /// Returns an error when the kev decision model cannot load, when its decision context cannot be created, and when the decision is not prepared or admitted.
    pub fn advances_until_it_ends(&self) -> Result<Vec<Result<DecisionAdvance, DecisionError>>> {
        let sequences = if self.question_count > 1 {
            DECISION_SLOTS_MINIMUM
        } else {
            NonZeroU16::MIN
        };
        let mut decision_model_runtime = DecisionModelRuntime::load(&kev_0_8b(), sequences.get())?;

        decision_model_runtime
            .runtime
            .model_runtime_parameters
            .context_size = ModelDefaultOr::Explicit(self.cache_cells);

        let mut decision = repeated_state_decision(self.state_repetitions);

        decision.questions.truncate(self.question_count);

        let prepared_decision =
            decision_model_runtime.prepared_decision(decision, sequences, u32::MAX)?;
        let mut llama_context = decision_model_runtime
            .create_llama_context(u32::from(sequences.get()))
            .context("the decision context must be created")?;
        let mut batch = LlamaBatch::new(llama_context.limits());
        let AdmittedDecision {
            mut active_decision_request,
            decision_result_rx: _decision_result_rx,
            decision_stop_tx: _decision_stop_tx,
        } = AdmittedDecision::admit(
            &DecisionCapacityLedger::new(sequences.get()),
            prepared_decision,
        )
        .context("the decision must be admitted")?;
        let mut advances = Vec::new();

        loop {
            let advance = active_decision_request.advance(
                &mut DecisionDecoder {
                    batch: &mut batch,
                    llama_context: &mut llama_context,
                },
                None,
                None,
            );
            let is_continuing = matches!(advance, Ok(DecisionAdvance::Continuing));

            advances.push(advance);

            if !is_continuing {
                return Ok(advances);
            }
        }
    }
}
