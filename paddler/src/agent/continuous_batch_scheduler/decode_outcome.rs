use llama_cpp_bindings::DecodeError;

#[derive(Debug)]
pub enum DecodeOutcome {
    Decoded,
    NeedsEviction,
    Aborted,
    Errored(i32),
}

impl DecodeOutcome {
    #[must_use]
    pub const fn from_decode_result(result: &Result<(), DecodeError>) -> Self {
        match result {
            Ok(()) => Self::Decoded,
            Err(DecodeError::NoKvCacheSlot) => Self::NeedsEviction,
            Err(DecodeError::Aborted | DecodeError::NTokensZero) => Self::Aborted,
            Err(DecodeError::Unknown(error_code)) => Self::Errored(*error_code),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::DecodeError;

    use super::DecodeOutcome;

    #[test]
    fn ok_maps_to_decoded() {
        assert!(matches!(
            DecodeOutcome::from_decode_result(&Ok(())),
            DecodeOutcome::Decoded
        ));
    }

    #[test]
    fn no_kv_cache_slot_maps_to_needs_eviction() {
        assert!(matches!(
            DecodeOutcome::from_decode_result(&Err(DecodeError::NoKvCacheSlot)),
            DecodeOutcome::NeedsEviction
        ));
    }

    #[test]
    fn aborted_maps_to_aborted() {
        assert!(matches!(
            DecodeOutcome::from_decode_result(&Err(DecodeError::Aborted)),
            DecodeOutcome::Aborted
        ));
    }

    #[test]
    fn n_tokens_zero_maps_to_aborted() {
        assert!(matches!(
            DecodeOutcome::from_decode_result(&Err(DecodeError::NTokensZero)),
            DecodeOutcome::Aborted
        ));
    }

    #[test]
    fn unknown_carries_error_code() {
        let outcome = DecodeOutcome::from_decode_result(&Err(DecodeError::Unknown(42)));

        assert!(matches!(outcome, DecodeOutcome::Errored(42)));
    }
}
