use llama_cpp_bindings::error::EvalMultimodalChunksError;
use llama_cpp_bindings::mtmd::MtmdEvalError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::oversized_image_details::OversizedImageDetails;

pub enum MultimodalIngestOutcome {
    Ingested(i32),
    Rejected(GeneratedTokenResult),
}

#[must_use]
pub fn multimodal_ingest_outcome(
    eval_outcome: Result<i32, EvalMultimodalChunksError>,
    agent_name: Option<&str>,
) -> MultimodalIngestOutcome {
    match eval_outcome {
        Ok(tokens_ingested) => MultimodalIngestOutcome::Ingested(tokens_ingested),
        Err(EvalMultimodalChunksError::EvalFailed(MtmdEvalError::ImageChunkExceedsBatchSize(
            mismatch,
        ))) => MultimodalIngestOutcome::Rejected(GeneratedTokenResult::ImageExceedsBatchSize(
            OversizedImageDetails {
                image_tokens: mismatch.image_tokens,
                n_batch: mismatch.n_batch,
            },
        )),
        Err(err) => MultimodalIngestOutcome::Rejected(GeneratedTokenResult::SamplerError(format!(
            "{agent_name:?}: failed to ingest multimodal prompt: {err}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::error::EvalMultimodalChunksError;
    use llama_cpp_bindings::mtmd::MtmdEvalError;
    use llama_cpp_bindings::mtmd::image_chunk_batch_size_mismatch::ImageChunkBatchSizeMismatch;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    use super::MultimodalIngestOutcome;
    use super::multimodal_ingest_outcome;

    #[test]
    fn a_successful_eval_reports_the_ingested_token_count() {
        let outcome = multimodal_ingest_outcome(Ok(42), Some("agent"));

        assert!(matches!(
            outcome,
            MultimodalIngestOutcome::Ingested(tokens_ingested) if tokens_ingested == 42
        ));
    }

    #[test]
    fn an_oversized_image_chunk_is_rejected_with_image_exceeds_batch_size() {
        let outcome = multimodal_ingest_outcome(
            Err(EvalMultimodalChunksError::EvalFailed(
                MtmdEvalError::ImageChunkExceedsBatchSize(ImageChunkBatchSizeMismatch {
                    image_tokens: 4096,
                    n_batch: 512,
                }),
            )),
            Some("agent"),
        );

        assert!(matches!(
            outcome,
            MultimodalIngestOutcome::Rejected(GeneratedTokenResult::ImageExceedsBatchSize(details))
                if details.image_tokens == 4096 && details.n_batch == 512
        ));
    }

    #[test]
    fn any_other_eval_error_is_rejected_as_a_sampler_error() {
        let outcome = multimodal_ingest_outcome(
            Err(EvalMultimodalChunksError::ChunkOutOfBounds(3)),
            Some("agent"),
        );

        assert!(matches!(
            outcome,
            MultimodalIngestOutcome::Rejected(ref generated)
                if discriminant(generated)
                    == discriminant(&GeneratedTokenResult::SamplerError(String::new()))
        ));
    }
}
