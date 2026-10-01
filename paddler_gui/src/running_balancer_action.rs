#[derive(Debug, Eq, PartialEq)]
pub enum RunningBalancerAction {
    None,
    CopyToClipboard(String),
    OpenUrl(String),
}
