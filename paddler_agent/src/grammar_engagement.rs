#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrammarEngagement {
    Immediate,
    AfterReasoning,
}

impl GrammarEngagement {
    #[must_use]
    pub const fn for_thinking(enable_thinking: bool) -> Self {
        if enable_thinking {
            Self::AfterReasoning
        } else {
            Self::Immediate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GrammarEngagement;

    #[test]
    fn thinking_defers_the_grammar_until_reasoning_closes() {
        assert_eq!(
            GrammarEngagement::for_thinking(true),
            GrammarEngagement::AfterReasoning
        );
    }

    #[test]
    fn without_thinking_the_grammar_applies_from_the_first_token() {
        assert_eq!(
            GrammarEngagement::for_thinking(false),
            GrammarEngagement::Immediate
        );
    }
}
