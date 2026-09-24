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
    use crate::embedding_batch_rejection::EmbeddingBatchRejection;

    #[test]
    fn accepts_batches_when_embeddings_are_enabled() {
        assert!(require_embeddings_enabled(true).is_ok());
    }

    #[test]
    fn rejects_batches_when_embeddings_are_disabled() {
        assert!(matches!(
            require_embeddings_enabled(false),
            Err(EmbeddingBatchRejection::EmbeddingsDisabled)
        ));
    }
}
