pub enum EmitTokenOutcome {
    Emitted(String),
    PieceConversionFailed(String),
    ChannelDropped,
}
