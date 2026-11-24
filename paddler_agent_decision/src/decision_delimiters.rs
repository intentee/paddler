#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionDelimiters<'text> {
    pub decide: &'text str,
    pub option_end: &'text str,
    pub option_start: &'text str,
    pub question: &'text str,
    pub state: &'text str,
}
