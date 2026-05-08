use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub enum ThinkingMode {
    On,
    Off,
}

impl ThinkingMode {
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::On)
    }
}
