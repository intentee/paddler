use std::collections::VecDeque;

use llama_cpp_bindings::token::LlamaToken;

use crate::decision_delimiter_tokens::DecisionDelimiterTokens;
use crate::decision_question_row::DecisionQuestionRow;
use crate::tokenized_decision_question::TokenizedDecisionQuestion;

fn question_row(
    delimiter_tokens: &DecisionDelimiterTokens,
    TokenizedDecisionQuestion {
        id,
        instructions,
        options,
    }: TokenizedDecisionQuestion,
) -> DecisionQuestionRow {
    let mut tokens = Vec::with_capacity(
        instructions.len() + options.iter().map(|option| option.len() + 2).sum::<usize>() + 2,
    );
    let mut option_end_offsets = Vec::with_capacity(options.len());

    tokens.push(delimiter_tokens.question);
    tokens.extend(instructions);

    for option in options {
        tokens.push(delimiter_tokens.option_start);
        tokens.extend(option);
        option_end_offsets.push(tokens.len());
        tokens.push(delimiter_tokens.option_end);
    }

    tokens.push(delimiter_tokens.decide);

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
        let mut state_tokens = Vec::with_capacity(state.len() + 1);

        state_tokens.push(delimiter_tokens.state);
        state_tokens.extend(state);

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
    pub fn needs_question_lane(&self) -> bool {
        !self.remaining_questions.is_empty()
    }

    #[must_use]
    pub fn required_cells(&self) -> usize {
        self.state.len()
            + self
                .remaining_questions
                .iter()
                .map(|question| question.tokens.len())
                .fold(self.first_question.tokens.len(), usize::max)
    }

    #[must_use]
    pub fn total_tokens(&self) -> usize {
        self.state.len()
            + self.first_question.tokens.len()
            + self
                .remaining_questions
                .iter()
                .map(|question| question.tokens.len())
                .sum::<usize>()
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
    fn a_single_question_is_the_first_question_and_needs_no_question_lane() {
        let layout = DecisionTokenLayout::new(
            &delimiter_tokens(),
            tokens(&[10]),
            Vec::new(),
            last_question(),
        );

        assert_eq!(layout.first_question.id, "last");
        assert!(!layout.needs_question_lane());
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
