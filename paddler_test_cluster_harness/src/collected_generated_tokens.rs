use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::token_result_with_producer::TokenResultWithProducer;

pub struct CollectedGeneratedTokens {
    pub text: String,
    pub token_results: Vec<TokenResultWithProducer>,
}

impl CollectedGeneratedTokens {
    #[must_use]
    pub fn into_token_results(self) -> Vec<GeneratedTokenResult> {
        self.token_results
            .into_iter()
            .map(|token_result_with_producer| token_result_with_producer.token_result)
            .collect()
    }

    #[must_use]
    pub fn content_text(&self) -> String {
        self.token_results
            .iter()
            .filter_map(|token_result_with_producer| {
                match &token_result_with_producer.token_result {
                    GeneratedTokenResult::ContentToken(piece) => Some(piece.as_str()),
                    _ => None,
                }
            })
            .collect()
    }

    #[must_use]
    pub fn reasoning_text(&self) -> String {
        self.token_results
            .iter()
            .filter_map(|token_result_with_producer| {
                match &token_result_with_producer.token_result {
                    GeneratedTokenResult::ReasoningToken(piece) => Some(piece.as_str()),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn summary(&self) -> Result<GenerationSummary, ClusterHarnessError> {
        match self
            .token_results
            .last()
            .map(|token_result_with_producer| &token_result_with_producer.token_result)
        {
            Some(GeneratedTokenResult::Done(summary)) => Ok(*summary),
            last_token_result => Err(ClusterHarnessError::GenerationEndedWithoutSummary {
                last_token_result: last_token_result.cloned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    use super::CollectedGeneratedTokens;
    use crate::cluster_harness_error::ClusterHarnessError;
    use crate::token_result_with_producer::TokenResultWithProducer;

    #[test]
    fn a_generation_ending_in_an_error_has_no_summary() {
        let collected = CollectedGeneratedTokens {
            text: String::new(),
            token_results: vec![TokenResultWithProducer {
                token_result: GeneratedTokenResult::SamplerError("failed".to_owned()),
                generated_by: None,
            }],
        };

        assert!(matches!(
            collected.summary(),
            Err(ClusterHarnessError::GenerationEndedWithoutSummary {
                last_token_result: Some(GeneratedTokenResult::SamplerError(_)),
            })
        ));
    }

    #[test]
    fn content_and_reasoning_text_keep_only_their_own_pieces() {
        let produced = |token_result| TokenResultWithProducer {
            token_result,
            generated_by: None,
        };
        let collected = CollectedGeneratedTokens {
            text: String::new(),
            token_results: vec![
                produced(GeneratedTokenResult::ReasoningToken("think".to_owned())),
                produced(GeneratedTokenResult::ContentToken("say".to_owned())),
                produced(GeneratedTokenResult::ReasoningToken("ing".to_owned())),
            ],
        };

        assert_eq!(
            [collected.reasoning_text(), collected.content_text()],
            ["thinking".to_owned(), "say".to_owned()]
        );
    }
}
