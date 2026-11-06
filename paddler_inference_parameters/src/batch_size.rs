use std::num::NonZeroU16;
use std::num::NonZeroU32;

use serde::Deserialize;
use serde::Serialize;

use crate::invalid_inference_parameters::InvalidInferenceParameters;

const DEFAULT_TOKENS: NonZeroU16 = NonZeroU16::new(2048).unwrap();
const MAX_TOKENS: u32 = i32::MAX.unsigned_abs();

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct BatchSize(NonZeroU32);

impl BatchSize {
    #[must_use]
    pub fn at_most(self, tokens: NonZeroU32) -> Self {
        Self(self.0.min(tokens))
    }

    #[must_use]
    pub const fn tokens(self) -> NonZeroU32 {
        self.0
    }

    #[must_use]
    pub const fn tokens_i32(self) -> i32 {
        self.0.get().cast_signed()
    }

    #[must_use]
    pub const fn tokens_usize(self) -> usize {
        self.0.get() as usize
    }
}

impl Default for BatchSize {
    fn default() -> Self {
        Self(NonZeroU32::from(DEFAULT_TOKENS))
    }
}

impl TryFrom<u32> for BatchSize {
    type Error = InvalidInferenceParameters;

    fn try_from(n_batch: u32) -> Result<Self, Self::Error> {
        if n_batch > MAX_TOKENS {
            return Err(InvalidInferenceParameters::BatchSizeTooLarge {
                n_batch,
                max_batch_size: MAX_TOKENS,
            });
        }

        NonZeroU32::new(n_batch)
            .map(Self)
            .ok_or(InvalidInferenceParameters::BatchSizeZero)
    }
}

impl From<BatchSize> for u32 {
    fn from(batch_size: BatchSize) -> Self {
        batch_size.0.get()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::BatchSize;
    use crate::invalid_inference_parameters::InvalidInferenceParameters;

    #[test]
    fn a_batch_never_exceeds_the_token_bound_it_is_capped_at() {
        assert_eq!(
            [
                BatchSize::default().at_most(NonZeroU32::new(512).unwrap()),
                BatchSize::default().at_most(NonZeroU32::new(4096).unwrap()),
            ],
            [BatchSize::try_from(512).unwrap(), BatchSize::default()]
        );
    }

    #[test]
    fn rejects_a_zero_batch() {
        assert_eq!(
            BatchSize::try_from(0),
            Err(InvalidInferenceParameters::BatchSizeZero)
        );
    }

    #[test]
    fn rejects_a_batch_llama_cpp_cannot_address() {
        assert_eq!(
            BatchSize::try_from(u32::MAX),
            Err(InvalidInferenceParameters::BatchSizeTooLarge {
                n_batch: u32::MAX,
                max_batch_size: i32::MAX.unsigned_abs(),
            })
        );
    }

    #[test]
    fn the_largest_batch_keeps_its_value_in_every_width() {
        let batch_size = BatchSize::try_from(i32::MAX.unsigned_abs()).unwrap();

        assert_eq!(batch_size.tokens_i32(), i32::MAX);
        assert_eq!(batch_size.tokens_usize(), i32::MAX.unsigned_abs() as usize);
        assert_eq!(u32::from(batch_size), batch_size.tokens().get());
    }
}
