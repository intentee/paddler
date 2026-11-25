#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversationRole {
    Assistant,
    System,
    User,
}

impl ConversationRole {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::User => "user",
        }
    }
}
