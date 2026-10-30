use paddler_messaging::generated_token_result::GeneratedTokenResult;

#[derive(Debug, Eq, PartialEq)]
pub enum ContinuousBatchTerminalOutcome {
    EmitNothing,
    EmitToClient(GeneratedTokenResult),
}
