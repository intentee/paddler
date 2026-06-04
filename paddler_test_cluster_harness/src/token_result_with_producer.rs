use paddler_messaging::generated_token_result::GeneratedTokenResult;

#[derive(Debug)]
pub struct TokenResultWithProducer {
    pub token_result: GeneratedTokenResult,
    pub generated_by: Option<String>,
}
