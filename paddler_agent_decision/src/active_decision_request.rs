use std::mem;
use std::time::Instant;

use llama_cpp_decision::decision_prompt::DecisionPrompt;
use llama_cpp_decision::decision_prompt_outputs::DecisionPromptOutputs;
use llama_cpp_mtmd::mtmd_context::MtmdContext;
use tokio::sync::mpsc;

use paddler_agent_runtime::receives_stop_request::ReceivesStopRequest as _;
use paddler_agent_runtime::sequence_id::SequenceId;
use paddler_agent_runtime::sequence_id_guard::SequenceIdGuard;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;

use crate::constants::DECISION_PROMPT_START;
use crate::decision_admission::DecisionAdmission;
use crate::decision_advance::DecisionAdvance;
use crate::decision_decode_step::DecisionDecodeStep;
use crate::decision_decoder::DecisionDecoder;
use crate::decision_error::DecisionError;
use crate::decision_plan::DecisionPlan;
use crate::decision_prompt_cursor::DecisionPromptCursor;
use crate::decision_prompt_outputs_request::DecisionPromptOutputsRequest;
use crate::decision_prompt_position::DecisionPromptPosition;
use crate::decision_prompt_span::DecisionPromptSpan;
use crate::decision_request_phase::DecisionRequestPhase;
use crate::question_lane::QuestionLane;
use crate::reports_decision_outcome::ReportsDecisionOutcome;
use crate::scheduled_decision_request::ScheduledDecisionRequest;

const fn prompt_sequence_id(
    question_lane: &QuestionLane,
    state_sequence: &SequenceIdGuard,
    is_last_prompt: bool,
) -> SequenceId {
    match question_lane {
        QuestionLane::Reserved(lane) if !is_last_prompt => lane.sequence_id(),
        QuestionLane::Released | QuestionLane::Reserved(_) => state_sequence.sequence_id(),
    }
}

fn release_question_lane(
    DecisionDecoder { llama_context, .. }: &DecisionDecoder,
    question_lane: &mut QuestionLane,
    agent_name: Option<&str>,
) {
    if let QuestionLane::Reserved(lane) = mem::replace(question_lane, QuestionLane::Released) {
        lane.sequence_id()
            .release_kv_cache_in(llama_context, agent_name);
    }
}

fn start_prompt(
    decoder: &DecisionDecoder,
    question_lane: &mut QuestionLane,
    state_sequence: &SequenceIdGuard,
    DecisionPromptPosition { is_first, is_last }: DecisionPromptPosition,
    agent_name: Option<&str>,
) -> Result<(), DecisionError> {
    match question_lane {
        QuestionLane::Reserved(lane) if !is_last => {
            decoder.copy_sequence(state_sequence.sequence_id(), lane.sequence_id())
        }
        QuestionLane::Reserved(_) => {
            release_question_lane(decoder, question_lane, agent_name);

            Ok(())
        }
        QuestionLane::Released if is_first => Ok(()),
        QuestionLane::Released => decoder.remove_sequence(state_sequence.sequence_id()),
    }
}

fn forward_answers(
    answered_questions: impl IntoIterator<Item = AnsweredDecisionQuestion>,
    decision_result_tx: &mpsc::UnboundedSender<DecisionResult>,
) -> Result<(), DecisionError> {
    answered_questions
        .into_iter()
        .try_for_each(|answered_question| {
            decision_result_tx
                .send(DecisionResult::QuestionAnswered(answered_question))
                .map_err(Box::new)
                .map_err(DecisionError::ClientDisconnected)
        })
}

fn final_outputs_of(prompt: &DecisionPrompt) -> DecisionPromptOutputsRequest {
    if prompt.outputs_every_token() {
        DecisionPromptOutputsRequest::EveryToken
    } else {
        DecisionPromptOutputsRequest::LastToken
    }
}

pub struct ActiveDecisionRequest {
    pub decision_result_tx: mpsc::UnboundedSender<DecisionResult>,
    pub decision_stop_rx: mpsc::UnboundedReceiver<()>,
    pub evaluated_tokens: usize,
    pub phase: DecisionRequestPhase,
    pub plan: DecisionPlan,
    pub prompt_index: usize,
    pub prompts_start: DecisionPromptCursor,
    pub question_lane: QuestionLane,
    pub slot_guard: SlotGuard,
    pub started_at: Instant,
    pub state_sequence: SequenceIdGuard,
}

impl ActiveDecisionRequest {
    #[must_use]
    pub fn new(
        ScheduledDecisionRequest {
            decision_result_tx,
            decision_stop_rx,
            plan,
            slot_guard,
            started_at,
        }: ScheduledDecisionRequest,
        DecisionAdmission {
            question_lane,
            state_sequence,
        }: DecisionAdmission,
    ) -> Self {
        let phase = if plan.shared_prefix_length > 0 {
            DecisionRequestPhase::DecodingSharedPrefix(DECISION_PROMPT_START)
        } else {
            DecisionRequestPhase::DecodingPrompt(DECISION_PROMPT_START)
        };

        Self {
            decision_result_tx,
            decision_stop_rx,
            evaluated_tokens: 0,
            phase,
            plan,
            prompt_index: 0,
            prompts_start: DECISION_PROMPT_START,
            question_lane,
            slot_guard,
            started_at,
            state_sequence,
        }
    }

    /// # Errors
    ///
    /// Returns the errors of [`DecisionPromptSpan::decode_step`] when a chunk of the prompt cannot be decoded, [`DecisionError::KvCacheRemovalFailed`] or [`DecisionError::KvCacheCopyFailed`] when a prompt cannot start from the shared tokens, [`DecisionError::AnswersUnreadable`] when the answers of a prompt cannot be read, and [`DecisionError::ClientDisconnected`] when the client stops listening for answers.
    pub fn advance(
        &mut self,
        decoder: &mut DecisionDecoder,
        projector: Option<&MtmdContext>,
        agent_name: Option<&str>,
    ) -> Result<DecisionAdvance, DecisionError> {
        let Self {
            decision_result_tx,
            evaluated_tokens,
            phase,
            plan,
            prompt_index,
            prompts_start,
            question_lane,
            state_sequence,
            ..
        } = self;
        let DecisionPlan {
            prepared,
            prompts,
            shared_prefix_length,
        } = plan;
        let Some(planned_prompt) = prompts.get(*prompt_index) else {
            return Ok(DecisionAdvance::Completed);
        };
        let (cursor, step) = match *phase {
            DecisionRequestPhase::DecodingSharedPrefix(cursor) => {
                let step = DecisionPromptSpan {
                    end: *shared_prefix_length,
                    final_outputs: DecisionPromptOutputsRequest::None,
                    projector,
                    planned_prompt,
                    sequence_id: state_sequence.sequence_id(),
                }
                .decode_step(decoder, cursor)?;

                *phase = match &step {
                    DecisionDecodeStep::Advanced(next_cursor) => {
                        DecisionRequestPhase::DecodingSharedPrefix(*next_cursor)
                    }
                    DecisionDecodeStep::Finished { next_cursor, .. } => {
                        *prompts_start = *next_cursor;

                        DecisionRequestPhase::DecodingPrompt(*next_cursor)
                    }
                };

                (cursor, step)
            }
            DecisionRequestPhase::DecodingPrompt(cursor) => {
                let prompt_position = DecisionPromptPosition::among(*prompt_index, prompts.len());

                if cursor == *prompts_start {
                    start_prompt(
                        decoder,
                        question_lane,
                        state_sequence,
                        prompt_position,
                        agent_name,
                    )?;
                }

                let sequence_id =
                    prompt_sequence_id(question_lane, state_sequence, prompt_position.is_last);
                let step = DecisionPromptSpan {
                    end: planned_prompt.prompt.tokens().len(),
                    final_outputs: final_outputs_of(&planned_prompt.prompt),
                    projector,
                    planned_prompt,
                    sequence_id,
                }
                .decode_step(decoder, cursor)?;

                *phase = match &step {
                    DecisionDecodeStep::Advanced(next_cursor) => {
                        DecisionRequestPhase::DecodingPrompt(*next_cursor)
                    }
                    DecisionDecodeStep::Finished {
                        output_batch_indices,
                        ..
                    } => {
                        let answers = prepared
                            .read_answers(
                                &planned_prompt.prompt,
                                decoder.llama_context,
                                DecisionPromptOutputs {
                                    batch_indices: output_batch_indices,
                                    sequence_id: sequence_id.llama_sequence_id(),
                                },
                            )
                            .map_err(DecisionError::AnswersUnreadable)?;

                        forward_answers(
                            answers
                                .into_iter()
                                .map(|(id, answer)| AnsweredDecisionQuestion { answer, id }),
                            decision_result_tx,
                        )?;

                        *prompt_index = prompt_index.saturating_add(1);

                        DecisionRequestPhase::DecodingPrompt(*prompts_start)
                    }
                };

                (cursor, step)
            }
        };

        *evaluated_tokens = evaluated_tokens.saturating_add(
            step.next_cursor()
                .token_index
                .saturating_sub(cursor.token_index),
        );

        Ok(DecisionAdvance::Continuing)
    }

    pub fn finish_with<TOutcome: ReportsDecisionOutcome>(
        mut self,
        decoder: &mut DecisionDecoder,
        agent_name: Option<&str>,
        outcome: TOutcome,
    ) {
        self.state_sequence
            .sequence_id()
            .release_kv_cache_in(decoder.llama_context, agent_name);
        release_question_lane(decoder, &mut self.question_lane, agent_name);

        outcome.report(agent_name, &self.decision_result_tx);
    }

    pub fn is_stop_requested(&mut self) -> bool {
        self.decision_stop_rx.is_stop_requested()
    }

    #[must_use]
    pub fn summary(&self) -> DecisionResult {
        DecisionResult::Done(DecisionSummary {
            input_tokens: self.evaluated_tokens,
            processing_milliseconds: self.started_at.elapsed().as_millis(),
        })
    }
}
