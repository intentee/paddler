use std::cmp::max_by_key;
use std::collections::VecDeque;
use std::iter::once;

use llama_cpp_bindings::token::LlamaToken;

use crate::decision_delimiter_tokens::DecisionDelimiterTokens;
use crate::decision_question_row::DecisionQuestionRow;
use crate::question_lane_demand::QuestionLaneDemand;
use crate::tokenized_decision_question::TokenizedDecisionQuestion;

fn question_row(
    &DecisionDelimiterTokens {
        decide,
        option_end,
        option_start,
        question,
        ..
    }: &DecisionDelimiterTokens,
    TokenizedDecisionQuestion {
        id,
        instructions,
        options,
    }: TokenizedDecisionQuestion,
) -> DecisionQuestionRow {
    let mut tokens = Vec::new();
    let mut option_end_offsets = Vec::with_capacity(options.len());

    tokens.push(question);
    tokens.extend(instructions);

    for option in options {
        tokens.push(option_start);
        tokens.extend(option);
        option_end_offsets.push(tokens.len());
        tokens.push(option_end);
    }

    tokens.push(decide);

    DecisionQuestionRow {
        id,
        option_end_offsets,
        tokens,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionTokenLayout {
    pub first_question: DecisionQuestionRow,
    pub remaining_questions: VecDeque<DecisionQuestionRow>,
    pub state: Vec<LlamaToken>,
}

impl DecisionTokenLayout {
    #[must_use]
    pub fn new(
        delimiter_tokens: &DecisionDelimiterTokens,
        state: Vec<LlamaToken>,
        leading_questions: Vec<TokenizedDecisionQuestion>,
        last_question: TokenizedDecisionQuestion,
    ) -> Self {
        let state_tokens = once(delimiter_tokens.state).chain(state).collect();
        let mut remaining_questions: VecDeque<DecisionQuestionRow> = leading_questions
            .into_iter()
            .map(|question| question_row(delimiter_tokens, question))
            .collect();
        let last_question = question_row(delimiter_tokens, last_question);
        let first_question = match remaining_questions.pop_front() {
            Some(first_question) => {
                remaining_questions.push_back(last_question);

                first_question
            }
            None => last_question,
        };

        Self {
            first_question,
            remaining_questions,
            state: state_tokens,
        }
    }

    #[must_use]
    pub fn question_lane_demand(&self) -> QuestionLaneDemand {
        if self.remaining_questions.is_empty() {
            QuestionLaneDemand::NotNeeded
        } else {
            QuestionLaneDemand::Needed
        }
    }

    #[must_use]
    pub fn required_cells(&self) -> usize {
        let longest_question = self.remaining_questions.iter().fold(
            &self.first_question,
            |longest_question, question| {
                max_by_key(
                    longest_question,
                    question,
                    |DecisionQuestionRow { tokens, .. }| tokens.len(),
                )
            },
        );

        self.state.iter().chain(&longest_question.tokens).count()
    }

    #[must_use]
    pub fn total_tokens(&self) -> usize {
        once(&self.state)
            .chain(once(&self.first_question.tokens))
            .chain(
                self.remaining_questions
                    .iter()
                    .map(|DecisionQuestionRow { tokens, .. }| tokens),
            )
            .map(Vec::len)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;

    use super::DecisionTokenLayout;
    use crate::decision_delimiter_tokens::DecisionDelimiterTokens;
    use crate::decision_question_row::DecisionQuestionRow;
    use crate::tokenized_decision_question::TokenizedDecisionQuestion;

    fn tokens(ids: &[i32]) -> Vec<LlamaToken> {
        ids.iter().copied().map(LlamaToken::new).collect()
    }

    fn delimiter_tokens() -> DecisionDelimiterTokens {
        DecisionDelimiterTokens {
            decide: LlamaToken::new(-5),
            option_end: LlamaToken::new(-4),
            option_start: LlamaToken::new(-3),
            question: LlamaToken::new(-2),
            state: LlamaToken::new(-1),
        }
    }

    fn first_question() -> TokenizedDecisionQuestion {
        TokenizedDecisionQuestion {
            id: "first".to_owned(),
            instructions: tokens(&[20]),
            options: vec![tokens(&[30, 31]), tokens(&[32])],
        }
    }

    fn last_question() -> TokenizedDecisionQuestion {
        TokenizedDecisionQuestion {
            id: "last".to_owned(),
            instructions: Vec::new(),
            options: vec![tokens(&[40])],
        }
    }

    fn two_question_layout() -> DecisionTokenLayout {
        DecisionTokenLayout::new(
            &delimiter_tokens(),
            tokens(&[10, 11]),
            vec![first_question()],
            last_question(),
        )
    }

    #[test]
    fn lays_out_the_state_and_one_row_per_question_with_kev_delimiters() {
        assert_eq!(
            two_question_layout(),
            DecisionTokenLayout {
                first_question: DecisionQuestionRow {
                    id: "first".to_owned(),
                    option_end_offsets: vec![5, 8],
                    tokens: tokens(&[-2, 20, -3, 30, 31, -4, -3, 32, -4, -5]),
                },
                remaining_questions: [DecisionQuestionRow {
                    id: "last".to_owned(),
                    option_end_offsets: vec![3],
                    tokens: tokens(&[-2, -3, 40, -4, -5]),
                }]
                .into(),
                state: tokens(&[-1, 10, 11]),
            }
        );
    }

    #[test]
    fn a_single_question_is_the_first_question() {
        let layout = DecisionTokenLayout::new(
            &delimiter_tokens(),
            tokens(&[10]),
            Vec::new(),
            last_question(),
        );

        assert_eq!(layout.first_question.id, "last");
        assert!(layout.remaining_questions.is_empty());
    }

    #[test]
    fn a_decision_needs_its_state_and_its_longest_question_in_the_cache() {
        assert_eq!(two_question_layout().required_cells(), 13);
    }

    #[test]
    fn a_decision_decodes_its_state_once_and_every_question_row() {
        assert_eq!(two_question_layout().total_tokens(), 18);
    }
}
