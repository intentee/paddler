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
}
