use std::cell::Cell;
use std::rc::Rc;

use paddler_agent_runtime::sequence_id_guard::SequenceIdGuard;
use paddler_agent_runtime::sequence_id_pool::SequenceIdPool;

use crate::decision_admission::DecisionAdmission;
use crate::decision_capacity_demand::DecisionCapacityDemand;
use crate::promised_decision_cells::PromisedDecisionCells;
use crate::question_lane::QuestionLane;

pub struct DecisionCapacityLedger {
    available_cells: Rc<Cell<usize>>,
    sequence_id_pool: SequenceIdPool,
}

impl DecisionCapacityLedger {
    #[must_use]
    pub fn new(context_cells: usize, sequences: u16) -> Self {
        Self {
            available_cells: Rc::new(Cell::new(context_cells)),
            sequence_id_pool: SequenceIdPool::new(sequences),
        }
    }

    #[must_use]
    pub fn admit(
        &self,
        DecisionCapacityDemand {
            needs_question_lane,
            required_cells,
        }: &DecisionCapacityDemand,
    ) -> Option<DecisionAdmission> {
        if self.available_cells.get() < *required_cells {
            return None;
        }

        let state_sequence = SequenceIdGuard::acquire(&self.sequence_id_pool)?;
        let question_lane = if *needs_question_lane {
            QuestionLane::Reserved(SequenceIdGuard::acquire(&self.sequence_id_pool)?)
        } else {
            QuestionLane::Released
        };

        self.available_cells
            .set(self.available_cells.get() - required_cells);

        Some(DecisionAdmission {
            promised_cells: PromisedDecisionCells {
                available_cells: self.available_cells.clone(),
                cells: *required_cells,
            },
            question_lane,
            state_sequence,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use llama_cpp_bindings::token::LlamaToken;

    use super::DecisionCapacityLedger;
    use crate::decision_capacity_demand::DecisionCapacityDemand;
    use crate::decision_question_row::DecisionQuestionRow;
    use crate::decision_token_layout::DecisionTokenLayout;

    fn question_row(row_tokens: i32) -> DecisionQuestionRow {
        DecisionQuestionRow {
            id: format!("question-{row_tokens}"),
            option_end_offsets: Vec::new(),
            tokens: (0..row_tokens).map(LlamaToken::new).collect(),
        }
    }

    fn capacity_demand(state_tokens: i32, question_row_tokens: &[i32]) -> DecisionCapacityDemand {
        let mut remaining_questions: VecDeque<DecisionQuestionRow> = question_row_tokens
            .iter()
            .copied()
            .map(question_row)
            .collect();

        DecisionTokenLayout {
            first_question: remaining_questions
                .pop_front()
                .expect("a test layout has at least one question"),
            remaining_questions,
            state: (0..state_tokens).map(LlamaToken::new).collect(),
        }
        .capacity_demand()
    }

    #[test]
    fn a_single_question_decision_takes_only_its_state_sequence() {
        let ledger = DecisionCapacityLedger::new(100, 2);
        let _first_admission = ledger
            .admit(&capacity_demand(10, &[5]))
            .expect("one free sequence fits a single question");

        assert!(ledger.admit(&capacity_demand(10, &[5])).is_some());
    }

    #[test]
    fn a_multi_question_decision_waits_for_a_free_question_lane() {
        let ledger = DecisionCapacityLedger::new(100, 1);

        assert!(ledger.admit(&capacity_demand(10, &[5, 5])).is_none());
    }

    #[test]
    fn a_decision_waits_while_every_sequence_is_taken() {
        let ledger = DecisionCapacityLedger::new(100, 2);
        let _first_admission = ledger.admit(&capacity_demand(10, &[5, 5]));

        assert!(ledger.admit(&capacity_demand(10, &[5])).is_none());
    }

    #[test]
    fn a_decision_waits_until_its_promised_cells_fit_and_runs_once_they_are_returned() {
        let ledger = DecisionCapacityLedger::new(20, 4);
        let first_admission = ledger.admit(&capacity_demand(10, &[5]));

        assert!(ledger.admit(&capacity_demand(5, &[3])).is_none());

        drop(first_admission);

        assert!(ledger.admit(&capacity_demand(5, &[3])).is_some());
    }
}
