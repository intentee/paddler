use llama_cpp_bindings::token::LlamaToken;
use paddler_messaging::generated_token_result::GeneratedTokenResult;

pub enum SampleOutcome {
    Sampled(LlamaToken),
    AllCandidatesEliminated,
    GrammarRejected(String),
    Failed(String),
}

impl SampleOutcome {
    /// # Errors
    /// Returns the terminal result the request ends with when no token could be sampled.
    pub fn into_sampled_token(self) -> Result<LlamaToken, GeneratedTokenResult> {
        match self {
            Self::Sampled(token) => Ok(token),
            Self::AllCandidatesEliminated => Err(GeneratedTokenResult::SamplerError(
                "all token candidates were eliminated during sampling".to_owned(),
            )),
            Self::GrammarRejected(message) => {
                Err(GeneratedTokenResult::GrammarRejectedModelOutput(message))
            }
            Self::Failed(message) => Err(GeneratedTokenResult::SamplerError(message)),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    use super::SampleOutcome;

    #[test]
    fn a_sampled_token_is_returned_as_is() {
        assert!(matches!(
            SampleOutcome::Sampled(LlamaToken::new(1)).into_sampled_token(),
            Ok(token) if token == LlamaToken::new(1)
        ));
    }

    #[test]
    fn exhausted_candidates_report_a_sampler_error() {
        assert!(matches!(
            SampleOutcome::AllCandidatesEliminated.into_sampled_token(),
            Err(GeneratedTokenResult::SamplerError(message))
                if message == "all token candidates were eliminated during sampling"
        ));
    }

    #[test]
    fn a_grammar_rejection_keeps_its_message() {
        assert!(matches!(
            SampleOutcome::GrammarRejected("no match".to_owned()).into_sampled_token(),
            Err(GeneratedTokenResult::GrammarRejectedModelOutput(message)) if message == "no match"
        ));
    }

    #[test]
    fn a_sampler_failure_keeps_its_message() {
        assert!(matches!(
            SampleOutcome::Failed("backend died".to_owned()).into_sampled_token(),
            Err(GeneratedTokenResult::SamplerError(message)) if message == "backend died"
        ));
    }
}
