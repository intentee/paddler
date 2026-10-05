use crate::token_result_with_producer::TokenResultWithProducer;

pub struct CollectedGeneratedTokens {
    pub text: String,
    pub token_results: Vec<TokenResultWithProducer>,
}
