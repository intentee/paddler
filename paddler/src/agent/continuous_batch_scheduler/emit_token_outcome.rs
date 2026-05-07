#[derive(Debug)]
pub enum EmitTokenOutcome {
    Emitted(String),
    ChannelDropped,
}
