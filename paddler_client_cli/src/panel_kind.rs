#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelKind {
    Thinking = 0,
    Response = 1,
    ToolCalls = 2,
    Undetermined = 3,
}

impl PanelKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Thinking => "Thinking",
            Self::Response => "Response",
            Self::ToolCalls => "Tool Calls",
            Self::Undetermined => "Undetermined",
        }
    }
}
