use paddler_messaging::generated_token_result::GeneratedTokenResult;

#[derive(Debug)]
pub enum ContinuousBatchTerminalOutcome {
    EmitNothing,
    EmitToClient(GeneratedTokenResult),
}
