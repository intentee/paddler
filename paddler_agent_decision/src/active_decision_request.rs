use std::collections::VecDeque;
use std::time::Instant;

use llama_cpp_bindings::token::LlamaToken;
use tokio::sync::mpsc;

use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_runtime::receives_stop_request::ReceivesStopRequest as _;
use paddler_agent_runtime::sequence_id_guard::SequenceIdGuard;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;

use crate::decision_admission::DecisionAdmission;
use crate::decision_advance::DecisionAdvance;
use crate::decision_decoder::DecisionDecoder;
use crate::decision_error::DecisionError;
use crate::decision_question_row::DecisionQuestionRow;
use crate::decision_request_phase::DecisionRequestPhase;
use crate::decision_token_layout::DecisionTokenLayout;
use crate::prepared_decision_request::PreparedDecisionRequest;
use crate::promised_decision_cells::PromisedDecisionCells;
use crate::question_lane::QuestionLane;

pub struct ActiveDecisionRequest {
    pub current_question: DecisionQuestionRow,
    pub decision_result_tx: mpsc::UnboundedSender<DecisionResult>,
    pub decision_stop_rx: mpsc::UnboundedReceiver<()>,
    pub hidden_states: Vec<Vec<f32>>,
    pub input_tokens: usize,
    pub phase: DecisionRequestPhase,
    pub promised_cells: PromisedDecisionCells,
    pub question_lane: QuestionLane,
    pub remaining_questions: VecDeque<DecisionQuestionRow>,
    pub slot_guard: SlotGuard,
    pub started_at: Instant,
    pub state: Vec<LlamaToken>,
    pub state_sequence: SequenceIdGuard,
}

impl ActiveDecisionRequest {
    #[must_use]
    pub fn new(
        PreparedDecisionRequest {
            decision_result_tx,
            decision_stop_rx,
            layout,
            slot_guard,
            started_at,
            ..
        }: PreparedDecisionRequest,
        DecisionAdmission {
            promised_cells,
            question_lane,
            state_sequence,
        }: DecisionAdmission,
    ) -> Self {
        let input_tokens = layout.total_tokens();
        let DecisionTokenLayout {
            first_question,
            remaining_questions,
            state,
        } = layout;

        Self {
            current_question: first_question,
            decision_result_tx,
            decision_stop_rx,
            hidden_states: Vec::new(),
            input_tokens,
            phase: DecisionRequestPhase::IngestingState { next_offset: 0 },
            promised_cells,
            question_lane,
            remaining_questions,
            slot_guard,
            started_at,
            state,
            state_sequence,
        }
    }

    pub fn advance(
        &mut self,
        decoder: &mut DecisionDecoder,
        pointer_head: &PointerHead,
    ) -> Result<DecisionAdvance, DecisionError> {
        match self.phase {
            DecisionRequestPhase::IngestingState { next_offset } => {
                let next_offset = decoder.ingest_state_chunk(
                    self.state_sequence.sequence_id(),
                    &self.state,
                    next_offset,
                )?;

                self.phase = if next_offset < self.state.len() {
                    DecisionRequestPhase::IngestingState { next_offset }
                } else {
                    DecisionRequestPhase::AnsweringQuestion { next_offset: 0 }
                };

                Ok(DecisionAdvance::Continuing)
            }
            DecisionRequestPhase::AnsweringQuestion { next_offset } => match &self.question_lane {
                QuestionLane::Reserved(lane) => {
                    let lane_sequence_id = lane.sequence_id();

                    self.answer_leading_question(
                        decoder,
                        pointer_head,
                        lane_sequence_id,
                        next_offset,
                    )
                }
                QuestionLane::Released => {
                    self.answer_last_question(decoder, pointer_head, next_offset)
                }
            },
        }
    }

    pub fn is_stop_requested(&mut self) -> bool {
        self.decision_stop_rx.is_stop_requested()
    }

    pub fn release_sequences(&self, decoder: &mut DecisionDecoder) -> Result<(), DecisionError> {
        decoder.remove_sequence(self.state_sequence.sequence_id())?;

        if let QuestionLane::Reserved(lane) = &self.question_lane {
            decoder.remove_sequence(lane.sequence_id())?;
        }

        Ok(())
    }

    #[must_use]
    pub fn summary(&self) -> DecisionResult {
        DecisionResult::Done(DecisionSummary {
            input_tokens: self.input_tokens,
            processing_milliseconds: self.started_at.elapsed().as_millis() as u64,
        })
    }

    fn answer_last_question(
        &mut self,
        decoder: &mut DecisionDecoder,
        pointer_head: &PointerHead,
        next_offset: usize,
    ) -> Result<DecisionAdvance, DecisionError> {
        let next_offset =
            self.decode_question_chunk(decoder, self.state_sequence.sequence_id(), next_offset)?;

        if next_offset < self.current_question.tokens.len() {
            self.phase = DecisionRequestPhase::AnsweringQuestion { next_offset };

            return Ok(DecisionAdvance::Continuing);
        }

        self.send_answer(pointer_head)?;

        Ok(self.finish_question())
    }

    fn answer_leading_question(
        &mut self,
        decoder: &mut DecisionDecoder,
        pointer_head: &PointerHead,
        lane_sequence_id: i32,
        next_offset: usize,
    ) -> Result<DecisionAdvance, DecisionError> {
        if next_offset == 0 {
            decoder.copy_sequence(self.state_sequence.sequence_id(), lane_sequence_id)?;
        }

        let next_offset = self.decode_question_chunk(decoder, lane_sequence_id, next_offset)?;

        if next_offset < self.current_question.tokens.len() {
            self.phase = DecisionRequestPhase::AnsweringQuestion { next_offset };

            return Ok(DecisionAdvance::Continuing);
        }

        self.send_answer(pointer_head)?;
        decoder.remove_sequence(lane_sequence_id)?;

        Ok(self.finish_question())
    }

    fn decode_question_chunk(
        &mut self,
        decoder: &mut DecisionDecoder,
        sequence_id: i32,
        next_offset: usize,
    ) -> Result<usize, DecisionError> {
        decoder.decode_row_chunk(
            sequence_id,
            &self.current_question,
            next_offset,
            self.state.len(),
            &mut self.hidden_states,
        )
    }

    fn finish_question(&mut self) -> DecisionAdvance {
        let Some(next_question) = self.remaining_questions.pop_front() else {
            return DecisionAdvance::Completed;
        };

        self.current_question = next_question;

        if self.remaining_questions.is_empty() {
            self.question_lane = QuestionLane::Released;
        }

        self.phase = DecisionRequestPhase::AnsweringQuestion { next_offset: 0 };

        DecisionAdvance::Continuing
    }

    fn send_answer(&mut self, pointer_head: &PointerHead) -> Result<(), DecisionError> {
        let probabilities = pointer_head.option_probabilities(&self.hidden_states);

        self.hidden_states.clear();
        self.decision_result_tx
            .send(DecisionResult::QuestionAnswered(DecisionAnswer {
                id: self.current_question.id.clone(),
                probabilities,
            }))
            .map_err(DecisionError::ClientDisconnected)
    }
}
