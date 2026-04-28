use paddler_types::generated_token_result::GeneratedTokenResult;

pub struct CollectedGeneratedTokens {
    pub text: String,
    pub token_results: Vec<GeneratedTokenResult>,
}
