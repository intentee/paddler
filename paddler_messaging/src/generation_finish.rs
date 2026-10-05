use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GenerationFinish {
    ContextFull,
    EndOfGeneration,
    MaxTokens,
    StopRequested,
}

impl GenerationFinish {
    #[must_use]
    pub const fn reached_a_length_limit(self) -> bool {
        match self {
            Self::ContextFull | Self::MaxTokens => true,
            Self::EndOfGeneration | Self::StopRequested => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GenerationFinish;

    #[test]
    fn only_a_full_context_or_the_token_budget_reach_a_length_limit() {
        assert_eq!(
            [
                GenerationFinish::ContextFull,
                GenerationFinish::EndOfGeneration,
                GenerationFinish::MaxTokens,
                GenerationFinish::StopRequested,
            ]
            .map(GenerationFinish::reached_a_length_limit),
            [true, false, true, false]
        );
    }
}
