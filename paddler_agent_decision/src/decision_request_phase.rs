use crate::decision_prompt_cursor::DecisionPromptCursor;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionRequestPhase {
    DecodingPrompt(DecisionPromptCursor),
    DecodingSharedPrefix(DecisionPromptCursor),
}
