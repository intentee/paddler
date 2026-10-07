use llama_cpp_bindings::token::LlamaToken;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionQuestionRow {
    pub id: String,
    pub option_end_offsets: Vec<usize>,
    pub tokens: Vec<LlamaToken>,
}

impl DecisionQuestionRow {
    #[must_use]
    pub fn is_output(&self, row_offset: usize) -> bool {
        row_offset + 1 == self.tokens.len() || self.option_end_offsets.contains(&row_offset)
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;

    use super::DecisionQuestionRow;

    #[test]
    fn option_ends_and_the_closing_decide_token_are_outputs() {
        let row = DecisionQuestionRow {
            id: "question".to_owned(),
            option_end_offsets: vec![2],
            tokens: (0..4).map(LlamaToken::new).collect(),
        };

        assert_eq!(
            (0..4)
                .map(|row_offset| row.is_output(row_offset))
                .collect::<Vec<_>>(),
            vec![false, false, true, true]
        );
    }
}
