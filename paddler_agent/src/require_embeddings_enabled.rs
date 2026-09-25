use crate::embedding_batch_rejection::EmbeddingBatchRejection;

pub const fn require_embeddings_enabled(
    enable_embeddings: bool,
) -> Result<(), EmbeddingBatchRejection> {
    if enable_embeddings {
        Ok(())
    } else {
        Err(EmbeddingBatchRejection::EmbeddingsDisabled)
    }
}

#[cfg(test)]
mod tests {
    use super::require_embeddings_enabled;

    fn rejection_message(enable_embeddings: bool) -> Result<(), String> {
        require_embeddings_enabled(enable_embeddings).map_err(|rejection| rejection.to_string())
    }

    #[test]
    fn accepts_batches_when_embeddings_are_enabled() {
        assert_eq!(rejection_message(true), Ok(()));
    }

    #[test]
    fn rejects_batches_when_embeddings_are_disabled() {
        assert_eq!(
            rejection_message(false),
            Err("embeddings are not enabled".to_owned())
        );
    }
}
