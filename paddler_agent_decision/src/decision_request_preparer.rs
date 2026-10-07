use std::time::Instant;

use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;
use paddler_messaging::request_params::decide_params::DecideParams;

use crate::decision_delimiter_tokens::DecisionDelimiterTokens;
use crate::decision_error::DecisionError;
use crate::decision_text_tokenizer::DecisionTextTokenizer;
use crate::decision_token_layout::DecisionTokenLayout;
use crate::prepared_decision_request::PreparedDecisionRequest;
use crate::tokenized_decision_question::TokenizedDecisionQuestion;

pub struct DecisionRequestPreparer {
    pub context_cells: u32,
    pub delimiter_tokens: DecisionDelimiterTokens,
    pub text_tokenizer: DecisionTextTokenizer,
}

impl DecisionRequestPreparer {
    fn tokenize_question(
        &self,
        DecisionQuestion {
            id,
            instructions,
            options,
        }: DecisionQuestion,
    ) -> Result<TokenizedDecisionQuestion, DecisionError> {
        Ok(TokenizedDecisionQuestion {
            id,
            instructions: self.text_tokenizer.tokenize(&instructions)?,
            options: options
                .iter()
                .map(|option| self.text_tokenizer.tokenize(option))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    fn prepare(
        &self,
        AgentRequest {
            params:
                DecideParams {
                    last_question,
                    leading_questions,
                    state,
                },
            response_tx: decision_result_tx,
            slot_guard,
            stop_rx: decision_stop_rx,
        }: AgentRequest<DecideParams, DecisionResult>,
    ) -> Result<PreparedDecisionRequest, DecisionError> {
        let started_at = Instant::now();
        let layout = DecisionTokenLayout::new(
            &self.delimiter_tokens,
            self.text_tokenizer.tokenize(&state)?,
            leading_questions
                .into_iter()
                .map(|question| self.tokenize_question(question))
                .collect::<Result<Vec<_>, _>>()?,
            self.tokenize_question(last_question)?,
        );
        let required_tokens = layout.required_cells();

        if required_tokens > self.context_cells as usize {
            return Err(DecisionError::RequestExceedsContext {
                details: OversizedDecisionDetails {
                    context_size: self.context_cells,
                    required_tokens,
                },
            });
        }

        Ok(PreparedDecisionRequest {
            decision_result_tx,
            decision_stop_rx,
            layout,
            slot_guard,
            started_at,
        })
    }
}

impl PreparesSchedulerCommand for DecisionRequestPreparer {
    type Command = PreparedDecisionRequest;
    type Request = AgentRequest<DecideParams, DecisionResult>;

    fn prepare_scheduler_command(
        &self,
        agent_name: Option<&str>,
        request: Self::Request,
    ) -> Option<Self::Command> {
        let decision_result_tx = request.response_tx.clone();

        match self.prepare(request) {
            Ok(prepared) => Some(prepared),
            Err(rejection) => {
                rejection.report(agent_name, &decision_result_tx);

                None
            }
        }
    }
}
