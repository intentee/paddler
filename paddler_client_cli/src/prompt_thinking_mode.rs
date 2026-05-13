use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub enum PromptThinkingMode {
    On,
    Off,
}

impl PromptThinkingMode {
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::On)
    }
}
