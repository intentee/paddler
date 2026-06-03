use llama_cpp_bindings::error::DecodeError;

#[derive(Debug)]
pub enum DecodeOutcome {
    Decoded,
    NeedsEviction,
    Aborted,
    Errored(DecodeError),
}

impl DecodeOutcome {
    #[must_use]
    pub fn from_decode_result(result: Result<(), DecodeError>) -> Self {
        match result {
            Ok(()) => Self::Decoded,
            Err(DecodeError::NoKvCacheSlot) => Self::NeedsEviction,
            Err(DecodeError::Aborted | DecodeError::BatchInvalid) => Self::Aborted,
            Err(other) => Self::Errored(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::error::DecodeError;

    use super::DecodeOutcome;

    #[test]
    fn ok_maps_to_decoded() {
        assert_eq!(
            discriminant(&DecodeOutcome::from_decode_result(Ok(()))),
            discriminant(&DecodeOutcome::Decoded)
        );
    }

    #[test]
    fn no_kv_cache_slot_maps_to_needs_eviction() {
        assert_eq!(
            discriminant(&DecodeOutcome::from_decode_result(Err(
                DecodeError::NoKvCacheSlot
            ))),
            discriminant(&DecodeOutcome::NeedsEviction)
        );
    }

    #[test]
    fn aborted_maps_to_aborted() {
        assert_eq!(
            discriminant(&DecodeOutcome::from_decode_result(Err(
                DecodeError::Aborted
            ))),
            discriminant(&DecodeOutcome::Aborted)
        );
    }

    #[test]
    fn batch_invalid_maps_to_aborted() {
        assert_eq!(
            discriminant(&DecodeOutcome::from_decode_result(Err(
                DecodeError::BatchInvalid
            ))),
            discriminant(&DecodeOutcome::Aborted)
        );
    }

    #[test]
    fn other_error_is_forwarded_as_errored() {
        let outcome =
            DecodeOutcome::from_decode_result(Err(DecodeError::UnknownStatus { code: 42 }));

        assert!(matches!(
            outcome,
            DecodeOutcome::Errored(DecodeError::UnknownStatus { code: 42 })
        ));
    }
}
