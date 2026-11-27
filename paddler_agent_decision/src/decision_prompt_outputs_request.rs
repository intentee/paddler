#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionPromptOutputsRequest {
    EveryToken,
    LastToken,
    None,
}

impl DecisionPromptOutputsRequest {
    #[must_use]
    pub const fn is_output(self, token_index: usize, last_token_index: usize) -> bool {
        match self {
            Self::EveryToken => true,
            Self::LastToken => token_index == last_token_index,
            Self::None => false,
        }
    }
}
